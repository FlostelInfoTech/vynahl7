//! End-to-End Performance Benchmark with Network I/O
//!
//! This benchmark measures realistic throughput including:
//! - MLLP network transmission (TCP)
//! - Message serialization/deserialization
//! - ACK generation and response handling
//!
//! Run with: cargo run --release --example e2e_benchmark
//!
//! This provides metrics comparable to industry interface engine benchmarks.

use vyna_hl7_core::{Field, Message, Segment};
use vyna_hl7_mllp::{testing::MockMllpServer, MllpClient};
use vyna_hl7_parser::parse_message;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// Test messages
const ADT_SMALL: &str = "MSH|^~\\&|HIS|Hospital|RIS|Radiology|20240315143000||ADT^A01^ADT_A01|MSG00001|P|2.5|||AL|NE\r\
PID|1||MRN12345^^^Hospital^MR||DOE^JOHN^ALLEN||19800315|M|||123 MAIN ST^^BOSTON^MA^02101^USA||^PRN^PH^^1^617^5551234\r\
PV1|1|I|ICU^101^A^Hospital||||1234567^SMITH^JANE^M^MD^^MD|||||||||VIP|||||||||||||||||||||||||20240315140000";

const ORU_MEDIUM: &str = "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315143000||ORU^R01^ORU_R01|MSG00002|P|2.5|||AL|NE\r\
PID|1||MRN12345^^^Hospital^MR||DOE^JOHN^ALLEN||19800315|M|||123 MAIN ST^^BOSTON^MA^02101^USA\r\
PV1|1|O|ER^201^B^Hospital||||1234567^SMITH^JANE^M^MD\r\
ORC|RE|ORD123456|LAB789012||CM||||20240315120000|||1234567^SMITH^JANE^M^MD\r\
OBR|1|ORD123456|LAB789012|CBC^Complete Blood Count^LN|||20240315110000|||||||||1234567^SMITH^JANE^M^MD||||||20240315120000|||F\r\
OBX|1|NM|WBC^White Blood Cell Count^LN||7.5|10*9/L|4.5-11.0|N|||F|||20240315115500\r\
OBX|2|NM|RBC^Red Blood Cell Count^LN||4.8|10*12/L|4.2-5.9|N|||F|||20240315115500\r\
OBX|3|NM|HGB^Hemoglobin^LN||14.5|g/dL|12.0-16.0|N|||F|||20240315115500\r\
OBX|4|NM|HCT^Hematocrit^LN||42.0|%|36.0-46.0|N|||F|||20240315115500\r\
OBX|5|NM|PLT^Platelet Count^LN||250|10*9/L|150-400|N|||F|||20240315115500";

/// Create an ACK message for a given message
fn create_ack(original: &Message) -> Message {
    let mut ack = Message::default();

    // MSH segment
    let mut msh = Segment::new("MSH");
    msh.fields.push(Field::from_value("|"));
    msh.fields.push(Field::from_value("^~\\&"));
    msh.fields.push(Field::from_value("SERVER"));
    msh.fields.push(Field::from_value("FACILITY"));
    msh.fields.push(Field::from_value("CLIENT"));
    msh.fields.push(Field::from_value("FACILITY"));
    msh.fields.push(Field::from_value("20240315143001"));
    msh.fields.push(Field::from_value("")); // Security
    msh.fields.push(Field::from_value("ACK^A01^ACK"));

    // Get message control ID from original
    let msg_id = original.segments[0]
        .fields
        .get(9)
        .and_then(|f| f.value())
        .unwrap_or("0");
    msh.fields.push(Field::from_value(format!("ACK{}", msg_id)));
    msh.fields.push(Field::from_value("P"));
    msh.fields.push(Field::from_value("2.5"));
    ack.segments.push(msh);

    // MSA segment
    let mut msa = Segment::new("MSA");
    msa.fields.push(Field::from_value("AA")); // Acknowledgment code
    msa.fields.push(Field::from_value(msg_id)); // Message control ID
    ack.segments.push(msa);

    ack
}

/// Benchmark configuration
struct BenchConfig {
    name: &'static str,
    message: &'static str,
    iterations: u32,
    warmup_iterations: u32,
    concurrent_clients: usize,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            name: "default",
            message: ADT_SMALL,
            iterations: 10000,
            warmup_iterations: 1000,
            concurrent_clients: 1,
        }
    }
}

/// Run a single-client benchmark
async fn run_single_client_benchmark(config: &BenchConfig) -> BenchResult {
    // Start mock server with ACK handler
    let server = MockMllpServer::new()
        .with_handler(|msg| Ok(create_ack(&msg)))
        .start()
        .await
        .expect("Failed to start server");

    let addr = server.url();

    // Connect client
    let mut client = MllpClient::connect(&addr)
        .await
        .expect("Failed to connect");

    // Parse the test message once
    let msg = parse_message(config.message).expect("Failed to parse message");

    // Warmup
    for _ in 0..config.warmup_iterations {
        let _ = client.send_message(&msg).await.expect("Warmup failed");
    }

    // Collect latencies
    let mut latencies: Vec<Duration> = Vec::with_capacity(config.iterations as usize);

    let start = Instant::now();

    for _ in 0..config.iterations {
        let iter_start = Instant::now();
        let _ = client.send_message(&msg).await.expect("Send failed");
        latencies.push(iter_start.elapsed());
    }

    let total_duration = start.elapsed();

    client.close().await.expect("Failed to close client");
    server.shutdown().await.expect("Failed to shutdown server");

    // Calculate statistics
    latencies.sort();
    let count = latencies.len();

    BenchResult {
        name: config.name.to_string(),
        message_size: config.message.len(),
        iterations: config.iterations,
        concurrent_clients: config.concurrent_clients,
        total_duration,
        min_latency: latencies[0],
        p50_latency: latencies[count * 50 / 100],
        p90_latency: latencies[count * 90 / 100],
        p95_latency: latencies[count * 95 / 100],
        p99_latency: latencies[count * 99 / 100],
        max_latency: latencies[count - 1],
    }
}

/// Run a multi-client concurrent benchmark
async fn run_concurrent_benchmark(config: &BenchConfig) -> BenchResult {
    // Start mock server with ACK handler
    let server = MockMllpServer::new()
        .with_handler(|msg| Ok(create_ack(&msg)))
        .start()
        .await
        .expect("Failed to start server");

    let addr = server.url();

    // Parse the test message once
    let msg = Arc::new(parse_message(config.message).expect("Failed to parse message"));

    let iterations_per_client = config.iterations / config.concurrent_clients as u32;
    let total_count = Arc::new(AtomicU64::new(0));
    let latencies = Arc::new(tokio::sync::Mutex::new(Vec::with_capacity(
        config.iterations as usize,
    )));

    // Warmup with single client
    {
        let mut client = MllpClient::connect(&addr)
            .await
            .expect("Failed to connect");
        for _ in 0..config.warmup_iterations {
            let _ = client.send_message(&msg).await.expect("Warmup failed");
        }
        client.close().await.ok();
    }

    let start = Instant::now();

    // Spawn concurrent clients
    let mut handles = Vec::new();
    for _ in 0..config.concurrent_clients {
        let addr = addr.clone();
        let msg = Arc::clone(&msg);
        let total_count = Arc::clone(&total_count);
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
                total_count.fetch_add(1, Ordering::Relaxed);
            }

            client.close().await.ok();

            // Merge local latencies
            let mut lat = latencies.lock().await;
            lat.extend(local_latencies);
        }));
    }

    // Wait for all clients to complete
    for handle in handles {
        handle.await.expect("Task failed");
    }

    let total_duration = start.elapsed();

    server.shutdown().await.expect("Failed to shutdown server");

    // Calculate statistics
    let mut latencies = Arc::try_unwrap(latencies)
        .expect("Arc still has references")
        .into_inner();
    latencies.sort();
    let count = latencies.len();

    BenchResult {
        name: config.name.to_string(),
        message_size: config.message.len(),
        iterations: total_count.load(Ordering::Relaxed) as u32,
        concurrent_clients: config.concurrent_clients,
        total_duration,
        min_latency: latencies[0],
        p50_latency: latencies[count * 50 / 100],
        p90_latency: latencies[count * 90 / 100],
        p95_latency: latencies[count * 95 / 100],
        p99_latency: latencies[count * 99 / 100],
        max_latency: latencies[count - 1],
    }
}

/// Benchmark result
struct BenchResult {
    name: String,
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
        println!("\n{}", "─".repeat(70));
        println!("  {} ({}B, {} clients)", self.name, self.message_size, self.concurrent_clients);
        println!("{}", "─".repeat(70));
        println!(
            "  Iterations:    {:>10}       Total Time:  {:>10.2?}",
            self.iterations, self.total_duration
        );
        println!(
            "  Throughput:    {:>10.0} msg/s",
            self.throughput()
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
        println!(
            "  Throughput @ p99: {:>10.0} msg/s",
            1.0 / self.p99_latency.as_secs_f64()
        );
    }
}

fn print_header() {
    println!();
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║       VynaHL7 End-to-End Performance Benchmark (with Network I/O)        ║");
    println!("╠══════════════════════════════════════════════════════════════════════╣");
    println!("║  Protocol:  MLLP over TCP (localhost)                                ║");
    println!("║  Includes:  Network I/O + Parse + ACK Generation + Response          ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
}

fn print_summary(results: &[BenchResult]) {
    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                           SUMMARY                                    ║");
    println!("╠══════════════════════════════════════════════════════════════════════╣");

    for r in results {
        println!(
            "║  {:30} {:>8.0} msg/s  (p99: {:>6.2}ms)       ║",
            r.name,
            r.throughput(),
            r.p99_latency.as_secs_f64() * 1000.0
        );
    }

    println!("╚══════════════════════════════════════════════════════════════════════╝");

    println!(
        "\n  VynaHL7 (single client):         {:>6.0} msg/s",
        results[0].throughput()
    );
    if results.len() > 2 {
        println!(
            "  VynaHL7 (4 concurrent clients):  {:>6.0} msg/s",
            results[2].throughput()
        );
    }
}

#[tokio::main]
async fn main() {
    print_header();

    let mut results = Vec::new();

    // Test 1: Single client, small ADT message
    println!("\n  Running: Single client, ADT^A01 (small)...");
    let result = run_single_client_benchmark(&BenchConfig {
        name: "ADT^A01 (single client)",
        message: ADT_SMALL,
        iterations: 10000,
        warmup_iterations: 1000,
        concurrent_clients: 1,
    })
    .await;
    result.print();
    results.push(result);

    // Test 2: Single client, medium ORU message
    println!("\n  Running: Single client, ORU^R01 (medium)...");
    let result = run_single_client_benchmark(&BenchConfig {
        name: "ORU^R01 (single client)",
        message: ORU_MEDIUM,
        iterations: 10000,
        warmup_iterations: 1000,
        concurrent_clients: 1,
    })
    .await;
    result.print();
    results.push(result);

    // Test 3: 4 concurrent clients, small ADT
    println!("\n  Running: 4 concurrent clients, ADT^A01...");
    let result = run_concurrent_benchmark(&BenchConfig {
        name: "ADT^A01 (4 clients)",
        message: ADT_SMALL,
        iterations: 20000,
        warmup_iterations: 1000,
        concurrent_clients: 4,
    })
    .await;
    result.print();
    results.push(result);

    // Test 4: 8 concurrent clients, small ADT
    println!("\n  Running: 8 concurrent clients, ADT^A01...");
    let result = run_concurrent_benchmark(&BenchConfig {
        name: "ADT^A01 (8 clients)",
        message: ADT_SMALL,
        iterations: 40000,
        warmup_iterations: 1000,
        concurrent_clients: 8,
    })
    .await;
    result.print();
    results.push(result);

    // Test 5: Sustained throughput simulation
    println!("\n  Running: Sustained throughput (30 seconds)...");
    let sustained_start = Instant::now();
    let mut sustained_count = 0u64;

    let server = MockMllpServer::new()
        .with_handler(|msg| Ok(create_ack(&msg)))
        .start()
        .await
        .expect("Failed to start server");

    let addr = server.url();
    let mut client = MllpClient::connect(&addr)
        .await
        .expect("Failed to connect");
    let msg = parse_message(ADT_SMALL).expect("Failed to parse message");

    // Warmup
    for _ in 0..1000 {
        let _ = client.send_message(&msg).await.ok();
    }

    let sustained_test_start = Instant::now();
    while sustained_test_start.elapsed() < Duration::from_secs(30) {
        if client.send_message(&msg).await.is_ok() {
            sustained_count += 1;
        }
    }

    let sustained_duration = sustained_start.elapsed();
    let sustained_throughput = sustained_count as f64 / sustained_test_start.elapsed().as_secs_f64();

    client.close().await.ok();
    server.shutdown().await.ok();

    println!("\n{}", "─".repeat(70));
    println!("  Sustained Throughput Test (30 seconds)");
    println!("{}", "─".repeat(70));
    println!("  Messages Processed: {:>10}", sustained_count);
    println!("  Duration:           {:>10.2?}", sustained_duration);
    println!("  Throughput:         {:>10.0} msg/s", sustained_throughput);
    println!(
        "  Projected Daily:    {:>10.1}M messages",
        sustained_throughput * 86400.0 / 1_000_000.0
    );

    print_summary(&results);
}
