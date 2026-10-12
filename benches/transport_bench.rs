//! VynaHL7 Transport Benchmark — MLLP and HTTP End-to-End Performance
//!
//! Run with:
//!   cargo bench --bench transport_bench --features "testing"

use vyna_hl7_core::{Field, Message, Segment};
use vyna_hl7_http::{testing::MockHttpServer, HttpClient};
use vyna_hl7_mllp::{testing::MockMllpServer, MllpClient};
use vyna_hl7_parser::parse_message;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

// ============================================================================
// Test Messages
// ============================================================================

const ADT_SMALL: &str = "\
MSH|^~\\&|HIS|HOSPITAL|EMR|HOSPITAL|20240315143000||ADT^A01^ADT_A01|MSG00001|P|2.5|||AL|NE\r\
PID|1||MRN12345^^^HOSPITAL^MR||DOE^JOHN^ALLEN||19800315|M|||123 MAIN ST^^BOSTON^MA^02101^USA||^PRN^PH^^1^617^5551234\r\
PV1|1|I|ICU^101^A^HOSPITAL||||1234567^SMITH^JANE^M^MD^^MD|||||||||VIP|||||||||||||||||||||||||20240315140000";

const ORU_MEDIUM: &str = "\
MSH|^~\\&|LAB|HOSPITAL|EMR|HOSPITAL|20240315143000||ORU^R01^ORU_R01|MSG00002|P|2.5|||AL|NE\r\
PID|1||MRN12345^^^HOSPITAL^MR||DOE^JOHN^ALLEN||19800315|M|||123 MAIN ST^^BOSTON^MA^02101^USA\r\
PV1|1|O|ER^201^B^HOSPITAL||||1234567^SMITH^JANE^M^MD\r\
ORC|RE|ORD123456|LAB789012||CM||||20240315120000|||1234567^SMITH^JANE^M^MD\r\
OBR|1|ORD123456|LAB789012|CBC^Complete Blood Count^LN|||20240315110000|||||||||1234567^SMITH^JANE^M^MD||||||20240315120000|||F\r\
OBX|1|NM|WBC^White Blood Cell Count^LN||7.5|10*9/L|4.5-11.0|N|||F|||20240315115500\r\
OBX|2|NM|RBC^Red Blood Cell Count^LN||4.8|10*12/L|4.2-5.9|N|||F|||20240315115500\r\
OBX|3|NM|HGB^Hemoglobin^LN||14.5|g/dL|12.0-16.0|N|||F|||20240315115500\r\
OBX|4|NM|PLT^Platelet Count^LN||250|10*9/L|150-400|N|||F|||20240315115500";

const RDE_PHARMACY: &str = "\
MSH|^~\\&|PHARMACY|HOSPITAL|EMR|HOSPITAL|20240315160000||RDE^O11^RDE_O11|MSG00003|P|2.5|||AL|NE\r\
PID|1||MRN12345^^^HOSPITAL^MR||DOE^JOHN^ALLEN||19800315|M|||123 MAIN ST^^BOSTON^MA^02101^USA\r\
PV1|1|I|ICU^101^A^HOSPITAL||||1234567^SMITH^JANE^M^MD\r\
ORC|NW|RX001|RXFILL001||A||||20240315160000|||1234567^SMITH^JANE^M^MD\r\
RXE|1|12345^Amoxicillin 500mg^NDC|500|MG|CAP^Capsule^HL70292|||||30|CAP^Capsule||||||||||||TAKE ONE CAPSULE THREE TIMES DAILY WITH FOOD\r\
TQ1|1||TID^Three Times Daily^HL70335|||7^D^HL70255\r\
RXR|PO^Oral^HL70162";

// ============================================================================
// ACK Generation
// ============================================================================

fn create_ack(original: &Message) -> Message {
    let mut ack = Message::default();

    let mut msh = Segment::new("MSH");
    msh.fields.push(Field::from_value("|"));
    msh.fields.push(Field::from_value("^~\\&"));
    msh.fields.push(Field::from_value("SERVER"));
    msh.fields.push(Field::from_value("FACILITY"));
    msh.fields.push(Field::from_value("CLIENT"));
    msh.fields.push(Field::from_value("FACILITY"));
    msh.fields.push(Field::from_value("20240315143001"));
    msh.fields.push(Field::from_value(""));
    msh.fields.push(Field::from_value("ACK^A01^ACK"));

    let msg_id = original.segments[0]
        .fields
        .get(9)
        .and_then(|f| f.value())
        .unwrap_or("0");
    msh.fields.push(Field::from_value(&format!("ACK{}", msg_id)));
    msh.fields.push(Field::from_value("P"));
    msh.fields.push(Field::from_value("2.5"));
    ack.segments.push(msh);

    let mut msa = Segment::new("MSA");
    msa.fields.push(Field::from_value("AA"));
    msa.fields.push(Field::from_value(msg_id));
    ack.segments.push(msa);

    ack
}

// ============================================================================
// Benchmark Result
// ============================================================================

#[allow(dead_code)]
struct BenchResult {
    name: String,
    protocol: String,
    message_type: String,
    message_size: usize,
    iterations: u32,
    concurrent_clients: usize,
    total_duration: Duration,
    min_latency: Duration,
    p50_latency: Duration,
    p90_latency: Duration,
    p95_latency: Duration,
    p99_latency: Duration,
    max_latency: Duration,
}

impl BenchResult {
    fn throughput(&self) -> f64 {
        self.iterations as f64 / self.total_duration.as_secs_f64()
    }

    fn print(&self) {
        println!();
        println!("  {}", "─".repeat(72));
        println!(
            "  {} | {} | {} ({}B) | {} client(s)",
            self.name, self.protocol, self.message_type, self.message_size, self.concurrent_clients
        );
        println!("  {}", "─".repeat(72));
        println!(
            "  Iterations:    {:>10}       Total Time:  {:>10.2?}",
            self.iterations, self.total_duration
        );
        println!(
            "  Throughput:    {:>10.0} msg/s   ({:.1} MB/s)",
            self.throughput(),
            self.throughput() * self.message_size as f64 / 1_048_576.0
        );
        println!();
        println!("  Latency Percentiles:");
        println!(
            "    min:   {:>10.3} ms    p50:   {:>10.3} ms",
            self.min_latency.as_secs_f64() * 1000.0,
            self.p50_latency.as_secs_f64() * 1000.0
        );
        println!(
            "    p90:   {:>10.3} ms    p95:   {:>10.3} ms",
            self.p90_latency.as_secs_f64() * 1000.0,
            self.p95_latency.as_secs_f64() * 1000.0
        );
        println!(
            "    p99:   {:>10.3} ms    max:   {:>10.3} ms",
            self.p99_latency.as_secs_f64() * 1000.0,
            self.max_latency.as_secs_f64() * 1000.0
        );
    }
}

fn compute_stats(
    name: &str,
    protocol: &str,
    message_type: &str,
    message_size: usize,
    concurrent_clients: usize,
    total_duration: Duration,
    latencies: &mut Vec<Duration>,
) -> BenchResult {
    latencies.sort();
    let count = latencies.len();

    BenchResult {
        name: name.to_string(),
        protocol: protocol.to_string(),
        message_type: message_type.to_string(),
        message_size,
        iterations: count as u32,
        concurrent_clients,
        total_duration,
        min_latency: latencies[0],
        p50_latency: latencies[count * 50 / 100],
        p90_latency: latencies[count * 90 / 100],
        p95_latency: latencies[count * 95 / 100],
        p99_latency: latencies[count * 99 / 100],
        max_latency: latencies[count - 1],
    }
}

// ============================================================================
// MLLP Benchmarks
// ============================================================================

async fn bench_mllp_single(
    msg_name: &str,
    raw_msg: &str,
    iterations: u32,
    warmup: u32,
) -> BenchResult {
    let server = MockMllpServer::new()
        .with_handler(|msg| Ok(create_ack(&msg)))
        .start()
        .await
        .expect("Failed to start MLLP server");

    let addr = server.url();
    let mut client = MllpClient::connect(&addr)
        .await
        .expect("Failed to connect MLLP client");

    let msg = parse_message(raw_msg).expect("Failed to parse message");

    // Warmup
    for _ in 0..warmup {
        let _ = client.send_message(&msg).await.expect("Warmup failed");
    }

    let mut latencies: Vec<Duration> = Vec::with_capacity(iterations as usize);
    let start = Instant::now();

    for _ in 0..iterations {
        let iter_start = Instant::now();
        let _ = client.send_message(&msg).await.expect("MLLP send failed");
        latencies.push(iter_start.elapsed());
    }

    let total_duration = start.elapsed();

    client.close().await.expect("Failed to close MLLP client");
    server
        .shutdown()
        .await
        .expect("Failed to shutdown MLLP server");

    compute_stats(
        "MLLP Single Client",
        "MLLP/TCP",
        msg_name,
        raw_msg.len(),
        1,
        total_duration,
        &mut latencies,
    )
}

async fn bench_mllp_concurrent(
    msg_name: &str,
    raw_msg: &'static str,
    iterations: u32,
    warmup: u32,
    num_clients: usize,
) -> BenchResult {
    let server = MockMllpServer::new()
        .with_handler(|msg| Ok(create_ack(&msg)))
        .start()
        .await
        .expect("Failed to start MLLP server");

    let addr = server.url();
    let msg = Arc::new(parse_message(raw_msg).expect("Failed to parse message"));
    let iterations_per_client = iterations / num_clients as u32;

    // Warmup
    {
        let mut client = MllpClient::connect(&addr)
            .await
            .expect("Failed to connect");
        for _ in 0..warmup {
            let _ = client.send_message(&msg).await.expect("Warmup failed");
        }
        client.close().await.ok();
    }

    let latencies = Arc::new(tokio::sync::Mutex::new(Vec::with_capacity(
        iterations as usize,
    )));

    let start = Instant::now();

    let mut handles = Vec::new();
    for _ in 0..num_clients {
        let addr = addr.clone();
        let msg = Arc::clone(&msg);
        let latencies = Arc::clone(&latencies);

        handles.push(tokio::spawn(async move {
            let mut client = MllpClient::connect(&addr)
                .await
                .expect("Failed to connect");
            let mut local_latencies = Vec::with_capacity(iterations_per_client as usize);

            for _ in 0..iterations_per_client {
                let iter_start = Instant::now();
                let _ = client.send_message(&msg).await.expect("Send failed");
                local_latencies.push(iter_start.elapsed());
            }

            client.close().await.ok();
            latencies.lock().await.extend(local_latencies);
        }));
    }

    for handle in handles {
        handle.await.expect("Task failed");
    }

    let total_duration = start.elapsed();
    server
        .shutdown()
        .await
        .expect("Failed to shutdown MLLP server");

    let mut latencies = Arc::try_unwrap(latencies)
        .expect("Arc still has references")
        .into_inner();

    compute_stats(
        &format!("MLLP {} Clients", num_clients),
        "MLLP/TCP",
        msg_name,
        raw_msg.len(),
        num_clients,
        total_duration,
        &mut latencies,
    )
}

// ============================================================================
// HTTP Benchmarks
// ============================================================================

async fn bench_http_single(
    msg_name: &str,
    raw_msg: &str,
    iterations: u32,
    warmup: u32,
) -> BenchResult {
    let server = MockHttpServer::new()
        .with_handler(|msg| Ok(create_ack(&msg)))
        .start()
        .await
        .expect("Failed to start HTTP server");

    let url = server.url();
    let client = HttpClient::new(&url).expect("Failed to create HTTP client");

    let msg = parse_message(raw_msg).expect("Failed to parse message");

    // Warmup
    for _ in 0..warmup {
        let _ = client.send_message(&msg).await.expect("HTTP warmup failed");
    }

    let mut latencies: Vec<Duration> = Vec::with_capacity(iterations as usize);
    let start = Instant::now();

    for _ in 0..iterations {
        let iter_start = Instant::now();
        let _ = client.send_message(&msg).await.expect("HTTP send failed");
        latencies.push(iter_start.elapsed());
    }

    let total_duration = start.elapsed();

    server
        .shutdown()
        .await
        .expect("Failed to shutdown HTTP server");

    compute_stats(
        "HTTP Single Client",
        "HTTP/1.1",
        msg_name,
        raw_msg.len(),
        1,
        total_duration,
        &mut latencies,
    )
}

async fn bench_http_concurrent(
    msg_name: &str,
    raw_msg: &str,
    iterations: u32,
    warmup: u32,
    num_clients: usize,
) -> BenchResult {
    let server = MockHttpServer::new()
        .with_handler(|msg| Ok(create_ack(&msg)))
        .start()
        .await
        .expect("Failed to start HTTP server");

    let url = server.url();
    let msg = Arc::new(parse_message(raw_msg).expect("Failed to parse message"));
    let iterations_per_client = iterations / num_clients as u32;

    // Warmup
    {
        let client = HttpClient::new(&url).expect("Failed to create HTTP client");
        for _ in 0..warmup {
            let _ = client
                .send_message(&msg)
                .await
                .expect("HTTP warmup failed");
        }
    }

    let latencies = Arc::new(tokio::sync::Mutex::new(Vec::with_capacity(
        iterations as usize,
    )));

    let start = Instant::now();

    let mut handles = Vec::new();
    for _ in 0..num_clients {
        let url = url.clone();
        let msg = Arc::clone(&msg);
        let latencies = Arc::clone(&latencies);

        handles.push(tokio::spawn(async move {
            let client = HttpClient::new(&url).expect("Failed to create HTTP client");
            let mut local_latencies = Vec::with_capacity(iterations_per_client as usize);

            for _ in 0..iterations_per_client {
                let iter_start = Instant::now();
                let _ = client.send_message(&msg).await.expect("HTTP send failed");
                local_latencies.push(iter_start.elapsed());
            }

            latencies.lock().await.extend(local_latencies);
        }));
    }

    for handle in handles {
        handle.await.expect("Task failed");
    }

    let total_duration = start.elapsed();
    server
        .shutdown()
        .await
        .expect("Failed to shutdown HTTP server");

    let mut latencies = Arc::try_unwrap(latencies)
        .expect("Arc still has references")
        .into_inner();

    compute_stats(
        &format!("HTTP {} Clients", num_clients),
        "HTTP/1.1",
        msg_name,
        raw_msg.len(),
        num_clients,
        total_duration,
        &mut latencies,
    )
}

// ============================================================================
// Sustained Throughput Test
// ============================================================================

async fn bench_sustained(protocol: &str, duration_secs: u64) -> (u64, Duration) {
    let msg = parse_message(ADT_SMALL).expect("Failed to parse");
    let mut count = 0u64;

    if protocol == "MLLP" {
        let server = MockMllpServer::new()
            .with_handler(|msg| Ok(create_ack(&msg)))
            .start()
            .await
            .expect("Failed to start");

        let addr = server.url();
        let mut client = MllpClient::connect(&addr).await.expect("Failed to connect");

        // Warmup
        for _ in 0..500 {
            let _ = client.send_message(&msg).await.ok();
        }

        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(duration_secs) {
            if client.send_message(&msg).await.is_ok() {
                count += 1;
            }
        }
        let elapsed = start.elapsed();

        client.close().await.ok();
        server.shutdown().await.ok();
        (count, elapsed)
    } else {
        let server = MockHttpServer::new()
            .with_handler(|msg| Ok(create_ack(&msg)))
            .start()
            .await
            .expect("Failed to start");

        let url = server.url();
        let client = HttpClient::new(&url).expect("Failed to create client");

        // Warmup
        for _ in 0..500 {
            let _ = client.send_message(&msg).await.ok();
        }

        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(duration_secs) {
            if client.send_message(&msg).await.is_ok() {
                count += 1;
            }
        }
        let elapsed = start.elapsed();

        server.shutdown().await.ok();
        (count, elapsed)
    }
}

// ============================================================================
// Environment Info
// ============================================================================

fn cmd(c: &str, args: &[&str]) -> String {
    Command::new(c)
        .args(args)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

fn read_os() -> String {
    std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|c| {
            c.lines()
                .find(|l| l.starts_with("PRETTY_NAME="))
                .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string())
        })
        .unwrap_or_else(|| cmd("uname", &["-o"]))
}

fn read_cpu() -> String {
    std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|c| {
            c.lines()
                .find(|l| l.starts_with("model name"))
                .and_then(|l| l.split(':').nth(1))
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_else(|| cmd("sysctl", &["-n", "machdep.cpu.brand_string"]))
}

fn read_mem() -> String {
    std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|c| {
            c.lines()
                .find(|l| l.starts_with("MemTotal:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse::<u64>().ok())
                .map(|kb| format!("{:.1} GB", kb as f64 / 1_048_576.0))
        })
        .unwrap_or_else(|| "unknown".into())
}

// ============================================================================
// Report: 4-Section Output
// ============================================================================

fn print_section_1_purpose() {
    println!();
    println!("================================================================================");
    println!("  VynaHL7 Transport Benchmark Report");
    println!("================================================================================");
    println!();
    println!("  1. PURPOSE");
    println!("  ----------");
    println!("  Measure end-to-end HL7 message processing throughput and latency over real");
    println!("  network transports (MLLP over TCP, HTTP/1.1) on localhost. This benchmark");
    println!("  answers: \"How fast can VynaHL7 send, receive, parse, and acknowledge HL7 messages");
    println!("  over a network?\" — the metric that matters in production interface engines.");
    println!();
    println!("  The full round-trip includes:");
    println!("    Client: encode message → MLLP/HTTP frame → TCP send");
    println!("    Server: TCP receive → deframe → parse HL7 → generate ACK → frame → TCP send");
    println!("    Client: TCP receive → deframe → parse ACK → return");
    println!();
    println!("  Test methodology follows standard HL7 interface engine benchmarking practices");
    println!("  (T1 store-and-forward, T2 translation, T3 round-trip). Both client and server");
    println!("  are VynaHL7's own MLLP/HTTP implementations running on localhost.");
}

fn print_section_2_methodology() {
    println!();
    println!("  2. METHODOLOGY");
    println!("  ---------------");
    println!("  Transport:     MLLP/TCP (intra-org, persistent connection, VT/FS/CR framing)");
    println!("                 HTTP/1.1 (inter-org, reqwest client, axum server, keep-alive)");
    println!("  Network:       localhost (127.0.0.1), auto-allocated ephemeral port");
    println!("  Server:        VynaHL7 MllpServer / HttpServer with ACK handler");
    println!("  Messages:      3 types — ADT^A01 (admission), ORU^R01 (lab result), RDE^O11 (Rx)");
    println!("  Concurrency:   1, 4, 8 simultaneous clients (persistent connections)");
    println!("  Warmup:        1,000 messages discarded before measurement");
    println!("  Iterations:    10,000 (single), 20,000 (4c), 40,000 (8c) measured round-trips");
    println!("  Sustained:     15-second continuous send (single client, ADT^A01)");
    println!("  Timing:        std::time::Instant per round-trip; sorted for percentile analysis");
    println!("  Latency:       Wall-clock time from send_message() call to ACK received");
    println!("  Throughput:    iterations / total_wall_clock_time (includes all overhead)");
}

fn print_section_3_configuration(iterations: u32, warmup: u32, sustained_secs: u64) {
    println!();
    println!("  3. CONFIGURATION");
    println!("  -----------------");
    println!();
    println!("  Hardware:");
    println!("    OS:            {}", read_os());
    println!("    Kernel:        {}", cmd("uname", &["-r"]));
    println!("    CPU:           {}", read_cpu());
    println!("    Cores:         {}", cmd("nproc", &[]));
    println!("    RAM:           {}", read_mem());
    println!();
    println!("  Toolchain:");
    println!("    Rust:          {}", cmd("rustc", &["--version"]));
    println!("    Cargo:         {}", cmd("cargo", &["--version"]));
    let target = cmd("rustc", &["-vV"])
        .lines()
        .find(|l| l.starts_with("host:"))
        .map(|l| l.trim_start_matches("host:").trim().to_string())
        .unwrap_or_else(|| "unknown".into());
    println!("    Target:        {}", target);
    println!("    Profile:       release (opt-level=3, lto=true, codegen-units=1)");
    println!();
    println!("  Benchmark Parameters:");
    println!("    Iterations:    {} (single client per message type)", iterations);
    println!("    Warmup:        {} messages (discarded)", warmup);
    println!("    Sustained:     {} seconds continuous", sustained_secs);
    println!("    Concurrency:   1, 4, 8 clients");
    println!();
    println!("  Test Messages:");
    println!("    ADT^A01:       {}B  (patient admission — 3 segments)", ADT_SMALL.len());
    println!("    ORU^R01:       {}B  (lab results — 9 segments, 4 OBX)", ORU_MEDIUM.len());
    println!("    RDE^O11:       {}B  (pharmacy order — 7 segments, RXE/TQ1/RXR)", RDE_PHARMACY.len());
    println!();
    println!("  Reproduce:");
    println!("    cargo bench --bench transport_bench --features testing");
}

fn print_section_4_results(
    results: &[BenchResult],
    mllp_sustained: (u64, f64),
    http_sustained: (u64, f64),
) {
    println!();
    println!("  4. RESULTS");
    println!("  ----------");

    // -- MLLP results --
    println!();
    println!("  4a. MLLP Transport (TCP, persistent connection)");
    println!();
    println!(
        "  {:<12} {:>8} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}",
        "Test", "Clients", "msg/s", "MB/s", "p50(ms)", "p90(ms)", "p99(ms)", "max(ms)"
    );
    println!("  {}", "─".repeat(88));

    for r in results.iter().filter(|r| r.protocol == "MLLP/TCP") {
        println!(
            "  {:<12} {:>8} {:>10.0} {:>10.1} {:>10.3} {:>10.3} {:>10.3} {:>10.3}",
            r.message_type,
            r.concurrent_clients,
            r.throughput(),
            r.throughput() * r.message_size as f64 / 1_048_576.0,
            r.p50_latency.as_secs_f64() * 1000.0,
            r.p90_latency.as_secs_f64() * 1000.0,
            r.p99_latency.as_secs_f64() * 1000.0,
            r.max_latency.as_secs_f64() * 1000.0,
        );
    }

    // -- HTTP results --
    println!();
    println!("  4b. HTTP Transport (HTTP/1.1, keep-alive)");
    println!();
    println!(
        "  {:<12} {:>8} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}",
        "Test", "Clients", "msg/s", "MB/s", "p50(ms)", "p90(ms)", "p99(ms)", "max(ms)"
    );
    println!("  {}", "─".repeat(88));

    for r in results.iter().filter(|r| r.protocol == "HTTP/1.1") {
        println!(
            "  {:<12} {:>8} {:>10.0} {:>10.1} {:>10.3} {:>10.3} {:>10.3} {:>10.3}",
            r.message_type,
            r.concurrent_clients,
            r.throughput(),
            r.throughput() * r.message_size as f64 / 1_048_576.0,
            r.p50_latency.as_secs_f64() * 1000.0,
            r.p90_latency.as_secs_f64() * 1000.0,
            r.p99_latency.as_secs_f64() * 1000.0,
            r.max_latency.as_secs_f64() * 1000.0,
        );
    }

    // -- Sustained --
    println!();
    println!("  4c. Sustained Throughput (15-second continuous, single client, ADT^A01)");
    println!();
    println!(
        "  MLLP:  {:>8} messages  {:>8.0} msg/s  {:>6.1}M messages/day",
        mllp_sustained.0,
        mllp_sustained.1,
        mllp_sustained.1 * 86400.0 / 1_000_000.0
    );
    println!(
        "  HTTP:  {:>8} messages  {:>8.0} msg/s  {:>6.1}M messages/day",
        http_sustained.0,
        http_sustained.1,
        http_sustained.1 * 86400.0 / 1_000_000.0
    );

    // -- Protocol comparison --
    println!();
    println!("  4d. Protocol Comparison (single client, ADT^A01)");
    println!();
    let mllp_adt = results
        .iter()
        .find(|r| r.protocol == "MLLP/TCP" && r.message_type == "ADT^A01" && r.concurrent_clients == 1);
    let http_adt = results
        .iter()
        .find(|r| r.protocol == "HTTP/1.1" && r.message_type == "ADT^A01" && r.concurrent_clients == 1);
    if let (Some(m), Some(h)) = (mllp_adt, http_adt) {
        println!(
            "  MLLP is {:.1}x faster throughput than HTTP ({:.0} vs {:.0} msg/s)",
            m.throughput() / h.throughput(),
            m.throughput(),
            h.throughput()
        );
        println!(
            "  MLLP p50 latency is {:.1}x lower than HTTP ({:.3} vs {:.3} ms)",
            h.p50_latency.as_secs_f64() / m.p50_latency.as_secs_f64(),
            m.p50_latency.as_secs_f64() * 1000.0,
            h.p50_latency.as_secs_f64() * 1000.0,
        );
    }

    println!();
    println!("================================================================================");
    println!();
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    let iterations = 10000u32;
    let warmup = 1000u32;
    let sustained_secs = 15u64;

    print_section_1_purpose();
    print_section_2_methodology();
    print_section_3_configuration(iterations, warmup, sustained_secs);

    println!();
    println!("  Running benchmarks...");

    let mut results = Vec::new();

    // --- MLLP ---
    eprint!("    MLLP ADT^A01 (1c)...");
    let r = bench_mllp_single("ADT^A01", ADT_SMALL, iterations, warmup).await;
    eprintln!(" {:.0} msg/s", r.throughput());
    results.push(r);

    eprint!("    MLLP ORU^R01 (1c)...");
    let r = bench_mllp_single("ORU^R01", ORU_MEDIUM, iterations, warmup).await;
    eprintln!(" {:.0} msg/s", r.throughput());
    results.push(r);

    eprint!("    MLLP RDE^O11 (1c)...");
    let r = bench_mllp_single("RDE^O11", RDE_PHARMACY, iterations, warmup).await;
    eprintln!(" {:.0} msg/s", r.throughput());
    results.push(r);

    eprint!("    MLLP ADT^A01 (4c)...");
    let r = bench_mllp_concurrent("ADT^A01", ADT_SMALL, 20000, warmup, 4).await;
    eprintln!(" {:.0} msg/s", r.throughput());
    results.push(r);

    eprint!("    MLLP ADT^A01 (8c)...");
    let r = bench_mllp_concurrent("ADT^A01", ADT_SMALL, 40000, warmup, 8).await;
    eprintln!(" {:.0} msg/s", r.throughput());
    results.push(r);

    // --- HTTP ---
    eprint!("    HTTP ADT^A01 (1c)...");
    let r = bench_http_single("ADT^A01", ADT_SMALL, iterations, warmup).await;
    eprintln!(" {:.0} msg/s", r.throughput());
    results.push(r);

    eprint!("    HTTP ORU^R01 (1c)...");
    let r = bench_http_single("ORU^R01", ORU_MEDIUM, iterations, warmup).await;
    eprintln!(" {:.0} msg/s", r.throughput());
    results.push(r);

    eprint!("    HTTP RDE^O11 (1c)...");
    let r = bench_http_single("RDE^O11", RDE_PHARMACY, iterations, warmup).await;
    eprintln!(" {:.0} msg/s", r.throughput());
    results.push(r);

    eprint!("    HTTP ADT^A01 (4c)...");
    let r = bench_http_concurrent("ADT^A01", ADT_SMALL, 20000, warmup, 4).await;
    eprintln!(" {:.0} msg/s", r.throughput());
    results.push(r);

    eprint!("    HTTP ADT^A01 (8c)...");
    let r = bench_http_concurrent("ADT^A01", ADT_SMALL, 40000, warmup, 8).await;
    eprintln!(" {:.0} msg/s", r.throughput());
    results.push(r);

    // --- Sustained ---
    eprint!("    MLLP sustained ({}s)...", sustained_secs);
    let (mllp_count, mllp_dur) = bench_sustained("MLLP", sustained_secs).await;
    let mllp_tps = mllp_count as f64 / mllp_dur.as_secs_f64();
    eprintln!(" {:.0} msg/s", mllp_tps);

    eprint!("    HTTP sustained ({}s)...", sustained_secs);
    let (http_count, http_dur) = bench_sustained("HTTP", sustained_secs).await;
    let http_tps = http_count as f64 / http_dur.as_secs_f64();
    eprintln!(" {:.0} msg/s", http_tps);

    // --- Print results section ---
    print_section_4_results(&results, (mllp_count, mllp_tps), (http_count, http_tps));
}
