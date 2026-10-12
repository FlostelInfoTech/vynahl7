//! System environment information for benchmark reports.
//!
//! Collects CPU, memory, OS, Rust toolchain, and build profile details
//! to contextualize benchmark results.

use std::process::Command;

pub struct EnvInfo {
    pub os: String,
    pub kernel: String,
    pub cpu_model: String,
    pub cpu_cores_physical: String,
    pub cpu_cores_logical: String,
    pub cpu_freq: String,
    pub memory_total: String,
    pub rust_version: String,
    pub cargo_version: String,
    pub target_triple: String,
    pub profile: String,
    pub hostname: String,
}

impl EnvInfo {
    pub fn collect() -> Self {
        Self {
            os: read_os_pretty(),
            kernel: cmd_output("uname", &["-r"]),
            cpu_model: read_cpu_model(),
            cpu_cores_physical: read_cpu_physical_cores(),
            cpu_cores_logical: cmd_output("nproc", &[]),
            cpu_freq: read_cpu_freq(),
            memory_total: read_memory_total(),
            rust_version: cmd_output("rustc", &["--version"]),
            cargo_version: cmd_output("cargo", &["--version"]),
            target_triple: cmd_output("rustc", &["-vV"])
                .lines()
                .find(|l| l.starts_with("host:"))
                .map(|l| l.trim_start_matches("host:").trim().to_string())
                .unwrap_or_else(|| "unknown".into()),
            profile: "release (opt-level=3, lto=true, codegen-units=1)".into(),
            hostname: cmd_output("hostname", &[]),
        }
    }

    pub fn print_report(&self) {
        println!();
        println!("╔══════════════════════════════════════════════════════════════════════════╗");
        println!("║                    VynaHL7 Benchmark Environment Report                      ║");
        println!("╠══════════════════════════════════════════════════════════════════════════╣");
        println!("║  System                                                                  ║");
        println!("╠══════════════════════════════════════════════════════════════════════════╣");
        self.print_row("Hostname", &self.hostname);
        self.print_row("OS", &self.os);
        self.print_row("Kernel", &self.kernel);
        println!("╠══════════════════════════════════════════════════════════════════════════╣");
        println!("║  CPU                                                                     ║");
        println!("╠══════════════════════════════════════════════════════════════════════════╣");
        self.print_row("Model", &self.cpu_model);
        self.print_row("Physical Cores", &self.cpu_cores_physical);
        self.print_row("Logical Cores", &self.cpu_cores_logical);
        self.print_row("Frequency", &self.cpu_freq);
        println!("╠══════════════════════════════════════════════════════════════════════════╣");
        println!("║  Memory                                                                  ║");
        println!("╠══════════════════════════════════════════════════════════════════════════╣");
        self.print_row("Total RAM", &self.memory_total);
        println!("╠══════════════════════════════════════════════════════════════════════════╣");
        println!("║  Toolchain                                                               ║");
        println!("╠══════════════════════════════════════════════════════════════════════════╣");
        self.print_row("Rust", &self.rust_version);
        self.print_row("Cargo", &self.cargo_version);
        self.print_row("Target", &self.target_triple);
        self.print_row("Profile", &self.profile);
        println!("╚══════════════════════════════════════════════════════════════════════════╝");
        println!();
    }

    fn print_row(&self, label: &str, value: &str) {
        // Truncate value to fit in the box if needed
        let max_val_len = 54;
        let display_val = if value.len() > max_val_len {
            &value[..max_val_len]
        } else {
            value
        };
        println!("║  {:<18} {:>54}  ║", label, display_val);
    }
}

fn cmd_output(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

fn read_file_trimmed(path: &str) -> String {
    std::fs::read_to_string(path)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unknown".into())
}

fn read_os_pretty() -> String {
    // Try /etc/os-release first
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if let Some(val) = line.strip_prefix("PRETTY_NAME=") {
                return val.trim_matches('"').to_string();
            }
        }
    }
    cmd_output("uname", &["-o"])
}

fn read_cpu_model() -> String {
    if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
        for line in content.lines() {
            if line.starts_with("model name")
                && let Some(val) = line.split(':').nth(1) {
                    return val.trim().to_string();
                }
        }
    }
    // macOS fallback
    let out = cmd_output("sysctl", &["-n", "machdep.cpu.brand_string"]);
    if out != "unknown" {
        return out;
    }
    "unknown".into()
}

fn read_cpu_physical_cores() -> String {
    // Linux: count unique core ids
    if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
        let mut core_ids = std::collections::HashSet::new();
        for line in content.lines() {
            if line.starts_with("core id")
                && let Some(val) = line.split(':').nth(1) {
                    core_ids.insert(val.trim().to_string());
                }
        }
        if !core_ids.is_empty() {
            return core_ids.len().to_string();
        }
    }
    // macOS fallback
    let out = cmd_output("sysctl", &["-n", "hw.physicalcpu"]);
    if out != "unknown" {
        return out;
    }
    "unknown".into()
}

fn read_cpu_freq() -> String {
    // Linux: read current max frequency
    if let Ok(khz) =
        read_file_trimmed("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq").parse::<u64>()
    {
            let ghz = khz as f64 / 1_000_000.0;
            return format!("{:.2} GHz (max)", ghz);
        }
    // Try /proc/cpuinfo
    if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
        for line in content.lines() {
            if line.starts_with("cpu MHz")
                && let Some(val) = line.split(':').nth(1)
                    && let Ok(mhz) = val.trim().parse::<f64>() {
                        return format!("{:.2} GHz (current)", mhz / 1000.0);
                    }
        }
    }
    // macOS fallback
    let out = cmd_output("sysctl", &["-n", "hw.cpufrequency"]);
    if out != "unknown"
        && let Ok(hz) = out.parse::<u64>() {
            return format!("{:.2} GHz", hz as f64 / 1_000_000_000.0);
        }
    "unknown".into()
}

fn read_memory_total() -> String {
    // Linux
    if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2
                    && let Ok(kb) = parts[1].parse::<u64>() {
                        let gb = kb as f64 / 1_048_576.0;
                        return format!("{:.1} GB", gb);
                    }
            }
        }
    }
    // macOS fallback
    let out = cmd_output("sysctl", &["-n", "hw.memsize"]);
    if out != "unknown"
        && let Ok(bytes) = out.parse::<u64>() {
            return format!("{:.1} GB", bytes as f64 / 1_073_741_824.0);
        }
    "unknown".into()
}
