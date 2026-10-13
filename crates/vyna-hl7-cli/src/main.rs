use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use colored::Colorize;
use serde_json::json;
use std::fs;
use std::io::{self, Read};
use std::time::Duration;
use vyna_hl7_core::{CharacterSet, Version, diff::DiffKind};
use vyna_hl7_parser::parse_message;
use vyna_hl7_terser::Terser;
use vyna_hl7_validator::{Validator, schema_loader};

#[derive(Parser)]
#[command(name = "vynahl7")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Force input decoding to this character set, ignoring MSH-18
    /// (e.g. 8859/1). Only needed for peers that misdeclare their encoding.
    #[arg(long, global = true)]
    charset: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse an HL7 message and display its structure
    Parse {
        /// Input file path (use '-' for stdin)
        #[arg(value_name = "FILE")]
        input: String,

        /// Output format: text, json, pretty
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Show segment details
        #[arg(short, long)]
        detailed: bool,
    },

    /// Validate an HL7 message against standards
    Validate {
        /// Input file path (use '-' for stdin)
        #[arg(value_name = "FILE")]
        input: String,

        /// HL7 version to validate against (e.g., 2.5, 2.7)
        #[arg(long, value_name = "VERSION")]
        hl7_version: Option<String>,

        /// Output format: text, json
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Strict validation against the HL7 specification (default):
        /// all findings fail, including version drift
        #[arg(long, conflicts_with_all = ["standard", "lenient"])]
        strict: bool,

        /// Standard profile: strict except version drift warns (for feeds
        /// validated against a pinned version)
        #[arg(long, conflicts_with_all = ["strict", "lenient"])]
        standard: bool,

        /// Lenient profile: only structural problems fail, rest warns
        #[arg(long, conflicts_with_all = ["strict", "standard"])]
        lenient: bool,
    },

    /// Extract field values using Terser paths
    Extract {
        /// Input file path (use '-' for stdin)
        #[arg(value_name = "FILE")]
        input: String,

        /// Terser paths to extract (e.g., PID-5, OBX(1)-5)
        #[arg(required = true)]
        paths: Vec<String>,

        /// Output format: text, json
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Convert message to different formats
    Convert {
        /// Input file path (use '-' for stdin)
        #[arg(value_name = "FILE")]
        input: String,

        /// Output format: json, fhir
        #[arg(short, long, required = true)]
        to: String,

        /// Pretty-print output
        #[arg(short, long)]
        pretty: bool,
    },

    /// Display message information
    Info {
        /// Input file path (use '-' for stdin)
        #[arg(value_name = "FILE")]
        input: String,
    },

    /// Send a message and print the acknowledgment
    Send {
        /// Input file path (use '-' for stdin)
        #[arg(value_name = "FILE")]
        input: String,

        /// Destination: `mllp://host:port` or `http(s)://host[:port]/path`
        #[arg(long, value_name = "URL")]
        to: String,

        /// HTTP basic-auth username
        #[arg(long, value_name = "USER")]
        user: Option<String>,

        /// HTTP basic-auth password
        #[arg(long, value_name = "PASSWORD")]
        password: Option<String>,

        /// Overall timeout in seconds
        #[arg(long, value_name = "SECS", default_value = "30")]
        timeout: u64,

        /// Use TLS (requires --ca-cert; build with --features tls)
        #[arg(long)]
        tls: bool,

        /// CA certificate PEM file for TLS server verification
        #[arg(long, value_name = "PEM")]
        ca_cert: Option<String>,

        /// Client certificate PEM file for mutual TLS
        #[arg(long, value_name = "PEM")]
        client_cert: Option<String>,

        /// Client key PEM file for mutual TLS
        #[arg(long, value_name = "PEM")]
        client_key: Option<String>,

        /// TLS server name for SNI (MLLP; defaults to the host)
        #[arg(long, value_name = "NAME")]
        server_name: Option<String>,
    },

    /// Show structural differences between two messages
    Diff {
        /// Original message file
        #[arg(value_name = "OLD")]
        old: String,

        /// New message file
        #[arg(value_name = "NEW")]
        new: String,

        /// Output format: text, json
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Terser paths to ignore (comma-separated, e.g. MSH-7,MSH-10)
        #[arg(long, value_name = "PATHS")]
        ignore: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let charset = cli
        .charset
        .as_deref()
        .map(CharacterSet::from_msh18)
        .transpose()
        .context("Invalid --charset value")?;

    match cli.command {
        Commands::Parse {
            input,
            format,
            detailed,
        } => parse_command(&input, &format, detailed, charset)?,
        Commands::Validate {
            input,
            hl7_version,
            format,
            strict,
            standard,
            lenient,
        } => validate_command(
            &input,
            hl7_version.as_deref(),
            &format,
            strict,
            standard,
            lenient,
            charset,
        )?,
        Commands::Extract {
            input,
            paths,
            format,
        } => extract_command(&input, &paths, &format, charset)?,
        Commands::Convert { input, to, pretty } => convert_command(&input, &to, pretty, charset)?,
        Commands::Info { input } => info_command(&input, charset)?,
        Commands::Send {
            input,
            to,
            user,
            password,
            timeout,
            tls,
            ca_cert,
            client_cert,
            client_key,
            server_name,
        } => {
            send_command(
                &input,
                &to,
                user.as_deref(),
                password.as_deref(),
                timeout,
                tls,
                ca_cert.as_deref(),
                client_cert.as_deref(),
                client_key.as_deref(),
                server_name.as_deref(),
                charset,
            )
            .await?;
        }
        Commands::Diff {
            old,
            new,
            format,
            ignore,
        } => diff_command(&old, &new, &format, ignore.as_deref(), charset)?,
    }

    Ok(())
}

fn read_input(input: &str, charset: Option<CharacterSet>) -> Result<String> {
    let bytes = if input == "-" {
        let mut buffer = Vec::new();
        io::stdin()
            .read_to_end(&mut buffer)
            .context("Failed to read from stdin")?;
        buffer
    } else {
        fs::read(input).context(format!("Failed to read file: {}", input))?
    };
    match charset {
        Some(charset) => charset.decode(&bytes).context("Failed to decode input"),
        None => CharacterSet::decode_message(&bytes).context("Failed to decode input"),
    }
}

fn parse_command(
    input: &str,
    format: &str,
    detailed: bool,
    charset: Option<CharacterSet>,
) -> Result<()> {
    let content = read_input(input, charset)?;
    let message = parse_message(&content).context("Failed to parse HL7 message")?;

    // Try to load schema for field/component names
    let schema = {
        let version = message.get_version().unwrap_or(Version::V2_5);
        message.get_message_type().and_then(|(msg_type, trigger)| {
            schema_loader::load_schema(version, &msg_type, &trigger).ok()
        })
    };

    match format {
        "json" => {
            let json = json!({
                "version": message.get_version().map(|v| format!("{:?}", v)),
                "message_type": message.get_message_type().map(|(m, e)| format!("{}^{}", m, e)),
                "control_id": message.get_control_id(),
                "segment_count": message.segments.len(),
                "segments": message.segments.iter().map(|s| {
                    let seg_def = schema.as_ref().and_then(|sc| sc.segments.get(&s.id));
                    json!({
                        "id": s.id,
                        "name": seg_def.map(|d| d.name.as_str()),
                        "field_count": s.fields.len(),
                        "fields": s.fields.iter().enumerate().filter_map(|(field_idx, field)| {
                            let field_num = field_idx + 1;
                            // MSH-1 (field separator) and MSH-2 (encoding characters)
                            // contain delimiter chars - use raw value, not encode()
                            let encoded = if s.id == "MSH" && field_num <= 2 {
                                field.value().unwrap_or("").to_string()
                            } else {
                                field.encode(&message.delimiters)
                            };
                            if encoded.is_empty() {
                                return None;
                            }
                            let field_def = seg_def.and_then(|d| d.fields.get(&field_num));
                            let mut field_json = json!({
                                "position": field_num,
                                "value": encoded,
                            });
                            if let Some(fd) = field_def {
                                field_json["name"] = json!(fd.name);
                                field_json["data_type"] = json!(fd.data_type);
                                // Add component names for composite values
                                if let Some(components) = &fd.components {
                                    let comp_parts: Vec<&str> = encoded.split('^').collect();
                                    let comp_details: Vec<_> = comp_parts.iter().enumerate().filter_map(|(i, val)| {
                                        if val.is_empty() { return None; }
                                        let comp_num = format!("{}", i + 1);
                                        let comp_name = components.get(&comp_num).map(|c| c.name.as_str());
                                        Some(json!({
                                            "position": i + 1,
                                            "name": comp_name,
                                            "value": val,
                                        }))
                                    }).collect();
                                    if !comp_details.is_empty() {
                                        field_json["components"] = json!(comp_details);
                                    }
                                }
                            }
                            Some(field_json)
                        }).collect::<Vec<_>>(),
                    })
                }).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        "pretty" => {
            println!("{}", "Message Structure:".bold().green());
            println!(
                "  Version: {}",
                message
                    .get_version()
                    .map(|v| format!("{:?}", v))
                    .unwrap_or("Unknown".to_string())
                    .cyan()
            );
            println!(
                "  Type: {}",
                message
                    .get_message_type()
                    .map(|(m, e)| format!("{}^{}", m, e))
                    .unwrap_or("Unknown".to_string())
                    .cyan()
            );
            println!(
                "  Control ID: {}",
                message.get_control_id().unwrap_or("N/A").cyan()
            );
            println!(
                "  Segments: {}",
                message.segments.len().to_string().yellow()
            );

            if detailed {
                println!();
                for (idx, segment) in message.segments.iter().enumerate() {
                    let seg_def = schema.as_ref().and_then(|sc| sc.segments.get(&segment.id));
                    let seg_label = match seg_def {
                        Some(def) => format!("{} ({})", segment.id.bold(), def.name),
                        None => format!("{}", segment.id.bold()),
                    };
                    println!(
                        "  {}. {} - {} fields",
                        idx + 1,
                        seg_label,
                        segment.fields.len()
                    );
                    for (field_idx, field) in segment.fields.iter().enumerate() {
                        let field_num = field_idx + 1;
                        // MSH-1 (field separator) and MSH-2 (encoding characters)
                        // contain delimiter chars - use raw value, not encode()
                        let encoded = if segment.id == "MSH" && field_num <= 2 {
                            field.value().unwrap_or("").to_string()
                        } else {
                            field.encode(&message.delimiters)
                        };
                        if encoded.is_empty() {
                            continue;
                        }
                        let field_def = seg_def.and_then(|d| d.fields.get(&field_num));
                        match field_def {
                            Some(fd) => {
                                println!(
                                    "      {}-{} {} ({}): {}",
                                    segment.id,
                                    field_num,
                                    fd.name.yellow(),
                                    fd.data_type.bright_black(),
                                    encoded.cyan()
                                );
                                // Show component breakdown for composite types
                                if let Some(components) = &fd.components {
                                    let comp_parts: Vec<&str> = encoded.split('^').collect();
                                    if comp_parts.len() > 1 {
                                        for (i, val) in comp_parts.iter().enumerate() {
                                            if val.is_empty() {
                                                continue;
                                            }
                                            let comp_num = format!("{}", i + 1);
                                            let comp_name = components
                                                .get(&comp_num)
                                                .map(|c| c.name.as_str())
                                                .unwrap_or("?");
                                            println!(
                                                "        .{} {}: {}",
                                                i + 1,
                                                comp_name.bright_black(),
                                                val
                                            );
                                        }
                                    }
                                }
                            }
                            None => {
                                println!(
                                    "      {}-{}: {}",
                                    segment.id,
                                    field_num,
                                    encoded.bright_black()
                                );
                            }
                        }
                    }
                }
            }
        }
        _ => {
            // Default text format
            println!("HL7 Message Parsed Successfully");
            println!(
                "Version: {}",
                message
                    .get_version()
                    .map(|v| format!("{:?}", v))
                    .unwrap_or("Unknown".to_string())
            );
            println!(
                "Message Type: {}",
                message
                    .get_message_type()
                    .map(|(m, e)| format!("{}^{}", m, e))
                    .unwrap_or("Unknown".to_string())
            );
            println!("Control ID: {}", message.get_control_id().unwrap_or("N/A"));
            println!("Segments: {}", message.segments.len());

            if detailed {
                println!("\nSegment Details:");
                for segment in &message.segments {
                    let seg_def = schema.as_ref().and_then(|sc| sc.segments.get(&segment.id));
                    let seg_label = match seg_def {
                        Some(def) => format!("{} - {}", segment.id, def.name),
                        None => segment.id.clone(),
                    };
                    println!("  {} ({} fields)", seg_label, segment.fields.len());
                }
            }
        }
    }

    Ok(())
}

fn validate_command(
    input: &str,
    version: Option<&str>,
    format: &str,
    strict: bool,
    standard: bool,
    lenient: bool,
    charset: Option<CharacterSet>,
) -> Result<()> {
    let content = read_input(input, charset)?;
    let message = parse_message(&content).context("Failed to parse HL7 message")?;

    // Get version from parameter or message
    let hl7_version = if let Some(v) = version {
        match v {
            "2.1" => Version::V2_1,
            "2.2" => Version::V2_2,
            "2.3" => Version::V2_3,
            "2.3.1" => Version::V2_3_1,
            "2.4" => Version::V2_4,
            "2.5" => Version::V2_5,
            "2.5.1" => Version::V2_5_1,
            "2.6" => Version::V2_6,
            "2.7" => Version::V2_7,
            "2.7.1" => Version::V2_7_1,
            "2.8" => Version::V2_8,
            "2.8.1" => Version::V2_8_1,
            "2.8.2" => Version::V2_8_2,
            _ => message.get_version().unwrap_or(Version::V2_5),
        }
    } else {
        message.get_version().unwrap_or(Version::V2_5)
    };

    // Strict is the default spec-compliance posture (--strict restates it);
    // --standard forgives version drift, --lenient forgives everything but
    // structure.
    let _ = strict;
    let mode = if lenient {
        "lenient"
    } else if standard {
        "standard"
    } else {
        "strict"
    };
    let context = if lenient {
        vyna_hl7_validator::ValidationContext::lenient()
    } else if standard {
        vyna_hl7_validator::ValidationContext::standard()
    } else {
        vyna_hl7_validator::ValidationContext::strict()
    };
    // Select the message schema from MSH-9 so validation checks structure,
    // not just the envelope. ACK carries no trigger event; types without a
    // schema fall back to structure-only validation.
    let schema_key = message.get_message_type().map(|(msg_type, trigger)| {
        if msg_type == "ACK" {
            (msg_type, String::new())
        } else {
            (msg_type, trigger)
        }
    });
    let mut validator = Validator::new(hl7_version);
    let mut schema_applied = false;
    if let Some((msg_type, trigger)) = &schema_key
        && let Ok(schema_validator) = Validator::for_message_type(hl7_version, msg_type, trigger)
    {
        validator = schema_validator;
        schema_applied = true;
    }
    let validator = validator.with_context(context);
    let result = validator.validate(&message);

    match format {
        "json" => {
            let errors_json: Vec<_> = result
                .errors
                .iter()
                .map(|e| {
                    json!({
                        "location": e.location,
                        "message": e.message,
                        "type": format!("{:?}", e.error_type),
                    })
                })
                .collect();

            let warnings_json: Vec<_> = result
                .warnings
                .iter()
                .map(|w| {
                    json!({
                        "location": w.location,
                        "message": w.message,
                    })
                })
                .collect();

            let json = json!({
                "valid": result.is_valid(),
                "mode": mode,
                "error_count": result.errors.len(),
                "warning_count": result.warnings.len(),
                "schema_applied": schema_applied,
                "errors": errors_json,
                "warnings": warnings_json,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        _ => {
            if result.is_valid() {
                println!("{}", "✓ Message is valid".green().bold());
            } else {
                println!("{}", "✗ Message validation failed".red().bold());
            }
            if mode == "strict" {
                println!("Validation mode: strict (HL7 specification compliance)");
            } else if mode == "standard" {
                println!("Validation mode: standard (strict except version drift warns)");
            } else {
                println!("Validation mode: lenient (only structural problems fail)");
            }

            if !result.errors.is_empty() {
                println!("\n{}", "Errors:".red().bold());
                for error in &result.errors {
                    println!("  • {} - {}", error.location.yellow(), error.message.red());
                }
            }

            if !result.warnings.is_empty() {
                println!("\n{}", "Warnings:".yellow().bold());
                for warning in &result.warnings {
                    println!("  • {} - {}", warning.location.yellow(), warning.message);
                }
            }

            if !schema_applied {
                println!(
                    "\n{}",
                    "Note: no schema for this message type; structure-only validation.".yellow()
                );
            }

            if result.is_valid() && result.warnings.is_empty() {
                println!("\nNo issues found.");
            }
        }
    }

    // Validation failure must fail the command so scripts and CI can rely
    // on the exit code (README: exit 1 on validation failure).
    if !result.is_valid() {
        std::process::exit(1);
    }
    Ok(())
}

fn extract_command(
    input: &str,
    paths: &[String],
    format: &str,
    charset: Option<CharacterSet>,
) -> Result<()> {
    let content = read_input(input, charset)?;
    let message = parse_message(&content).context("Failed to parse HL7 message")?;
    let terser = Terser::new(&message);

    match format {
        "json" => {
            let mut results = serde_json::Map::new();
            for path in paths {
                // A malformed path is a usage error (stderr, nonzero
                // exit); a well-formed path with no value yields "".
                let value = match terser.get(path) {
                    Ok(value) => value.unwrap_or(""),
                    Err(e) => bail!("invalid Terser path '{path}': {e}"),
                };
                results.insert(path.clone(), json!(value));
            }
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
        _ => {
            for path in paths {
                let value = match terser.get(path) {
                    Ok(value) => value.unwrap_or(""),
                    Err(e) => bail!("invalid Terser path '{path}': {e}"),
                };
                if value.is_empty() {
                    println!("{}: {}", path.cyan(), "N/A".bright_black());
                } else {
                    println!("{}: {}", path.cyan(), value.green());
                }
            }
        }
    }

    Ok(())
}

fn convert_command(
    input: &str,
    to: &str,
    pretty: bool,
    charset: Option<CharacterSet>,
) -> Result<()> {
    let content = read_input(input, charset)?;
    let message = parse_message(&content).context("Failed to parse HL7 message")?;

    match to {
        "json" => {
            let json = json!({
                "version": message.get_version().map(|v| format!("{:?}", v)),
                "message_type": message.get_message_type().map(|(m, e)| format!("{}^{}", m, e)),
                "control_id": message.get_control_id(),
                "sending_application": message.get_sending_application(),
                "sending_facility": message.get_sending_facility(),
                "receiving_application": message.get_receiving_application(),
                "receiving_facility": message.get_receiving_facility(),
                "segments": message.segments.iter().map(|seg| {
                    json!({
                        "id": seg.id,
                        "fields": seg.fields.iter().map(|f| f.encode(&message.delimiters)).collect::<Vec<_>>(),
                    })
                }).collect::<Vec<_>>(),
            });

            if pretty {
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                println!("{}", serde_json::to_string(&json)?);
            }
        }
        "fhir" => {
            #[cfg(feature = "fhir")]
            {
                match vyna_hl7_fhir::bundle::convert_message_to_bundle(&message) {
                    Ok(bundle) => {
                        if pretty {
                            println!("{}", serde_json::to_string_pretty(&bundle)?);
                        } else {
                            println!("{}", serde_json::to_string(&bundle)?);
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", format!("FHIR conversion failed: {}", e).red());
                        std::process::exit(1);
                    }
                }
            }
            #[cfg(not(feature = "fhir"))]
            {
                eprintln!("{}", "FHIR conversion requires the 'fhir' feature. Rebuild with: cargo build --features fhir".red());
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("{}", format!("Unsupported output format: {}", to).red());
            std::process::exit(1);
        }
    }

    Ok(())
}

fn info_command(input: &str, charset: Option<CharacterSet>) -> Result<()> {
    let content = read_input(input, charset)?;
    let message = parse_message(&content).context("Failed to parse HL7 message")?;

    println!("{}", "HL7 Message Information".bold().green());
    println!("{}", "=".repeat(50).bright_black());

    println!("\n{}", "Header Information:".bold());
    println!(
        "  Version:              {}",
        message
            .get_version()
            .map(|v| format!("{:?}", v))
            .unwrap_or("Unknown".to_string())
            .cyan()
    );
    println!(
        "  Message Type:         {}",
        message
            .get_message_type()
            .map(|(m, e)| format!("{}^{}", m, e))
            .unwrap_or("Unknown".to_string())
            .cyan()
    );
    println!(
        "  Control ID:           {}",
        message.get_control_id().unwrap_or("N/A").cyan()
    );
    println!(
        "  Sending Application:  {}",
        message.get_sending_application().unwrap_or("N/A").cyan()
    );
    println!(
        "  Sending Facility:     {}",
        message.get_sending_facility().unwrap_or("N/A").cyan()
    );
    println!(
        "  Receiving Application: {}",
        message.get_receiving_application().unwrap_or("N/A").cyan()
    );
    println!(
        "  Receiving Facility:   {}",
        message.get_receiving_facility().unwrap_or("N/A").cyan()
    );

    println!("\n{}", "Message Structure:".bold());
    println!(
        "  Total Segments:       {}",
        message.segments.len().to_string().yellow()
    );

    // Count segment types
    let mut segment_counts = std::collections::HashMap::new();
    for segment in &message.segments {
        *segment_counts.entry(&segment.id).or_insert(0) += 1;
    }

    println!(
        "  Segment Types:        {}",
        segment_counts.len().to_string().yellow()
    );
    println!("\n  Segment Breakdown:");
    let mut sorted_segments: Vec<_> = segment_counts.iter().collect();
    sorted_segments.sort_by(|a, b| a.0.cmp(b.0));
    for (id, count) in sorted_segments {
        println!(
            "    {} x {}",
            count.to_string().bright_white(),
            id.bright_cyan()
        );
    }

    // Calculate message size
    let encoded = message.encode();
    println!("\n{}", "Size Information:".bold());
    println!(
        "  Encoded Size:         {} bytes",
        encoded.len().to_string().yellow()
    );
    println!(
        "  Average Segment Size: {} bytes",
        (encoded.len() / message.segments.len())
            .to_string()
            .yellow()
    );

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn send_command(
    input: &str,
    to: &str,
    user: Option<&str>,
    password: Option<&str>,
    timeout_secs: u64,
    tls: bool,
    ca_cert: Option<&str>,
    client_cert: Option<&str>,
    client_key: Option<&str>,
    server_name: Option<&str>,
    charset: Option<CharacterSet>,
) -> Result<()> {
    let content = read_input(input, charset)?;
    let message = parse_message(&content).context("Failed to parse HL7 message")?;
    let timeout = Duration::from_secs(timeout_secs.max(1));

    let ack = if let Some(addr) = to.strip_prefix("mllp://") {
        if user.is_some() || password.is_some() {
            bail!("--user/--password are HTTP-only; MLLP has no authentication");
        }
        send_mllp(
            &message,
            addr,
            timeout,
            tls,
            ca_cert,
            client_cert,
            client_key,
            server_name,
        )
        .await?
    } else if to.starts_with("http://") || to.starts_with("https://") {
        if server_name.is_some() {
            bail!("--server-name is MLLP-only");
        }
        send_http(
            &message,
            to,
            user,
            password,
            timeout,
            tls,
            ca_cert,
            client_cert,
            client_key,
        )
        .await?
    } else {
        bail!("--to must start with mllp://, http://, or https:// (got '{to}')");
    };

    println!("{}", ack.encode().replace('\r', "\n").trim_end());

    // Negative acknowledgments fail the command so scripts can rely on it.
    let terser = Terser::new(&ack);
    let code = terser.get("MSA-1").unwrap_or(None).unwrap_or_default();
    if !matches!(code, "AA" | "CA") {
        let text = terser.get("MSA-3").unwrap_or(None).unwrap_or_default();
        bail!("negative acknowledgment {code}: {text}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn send_mllp(
    message: &vyna_hl7_core::Message,
    addr: &str,
    timeout: Duration,
    tls: bool,
    ca_cert: Option<&str>,
    client_cert: Option<&str>,
    client_key: Option<&str>,
    server_name: Option<&str>,
) -> Result<vyna_hl7_core::Message> {
    use vyna_hl7_mllp::MllpClient;

    if !tls {
        if ca_cert.is_some() || client_cert.is_some() || client_key.is_some() {
            bail!("--ca-cert/--client-cert/--client-key require --tls");
        }
        let mut client = tokio::time::timeout(timeout, MllpClient::connect(addr))
            .await
            .context("MLLP connect timed out")?
            .context("MLLP connect failed")?;
        return tokio::time::timeout(timeout, client.send_message(message))
            .await
            .context("MLLP send timed out")?
            .context("MLLP send failed");
    }

    #[cfg(not(feature = "tls"))]
    {
        let _ = (ca_cert, client_cert, client_key, server_name, timeout);
        bail!("TLS support requires building with --features tls");
    }
    #[cfg(feature = "tls")]
    {
        use vyna_hl7_mllp::tls::TlsClientConfig;

        let ca = ca_cert.context("--tls requires --ca-cert")?;
        let tls_config = match (client_cert, client_key) {
            (Some(cert), Some(key)) => TlsClientConfig::with_mtls(ca, cert, key)?,
            (None, None) => TlsClientConfig::with_ca_cert(ca)?,
            _ => bail!("mutual TLS needs both --client-cert and --client-key"),
        };
        let name = server_name
            .map(str::to_string)
            .or_else(|| addr.rsplit(':').nth(1).map(str::to_string))
            .context("cannot determine TLS server name; pass --server-name")?;
        let mut client =
            tokio::time::timeout(timeout, MllpClient::connect_tls(addr, &name, tls_config))
                .await
                .context("MLLP+TLS connect timed out")?
                .context("MLLP+TLS connect failed")?;
        tokio::time::timeout(timeout, client.send_message(message))
            .await
            .context("MLLP send timed out")?
            .context("MLLP send failed")
    }
}

#[allow(clippy::too_many_arguments)]
async fn send_http(
    message: &vyna_hl7_core::Message,
    endpoint: &str,
    user: Option<&str>,
    password: Option<&str>,
    timeout: Duration,
    tls: bool,
    ca_cert: Option<&str>,
    client_cert: Option<&str>,
    client_key: Option<&str>,
) -> Result<vyna_hl7_core::Message> {
    use vyna_hl7_http::HttpClient;

    let mut client = HttpClient::new(endpoint).context("invalid HTTP endpoint")?;
    if let (Some(user), Some(password)) = (user, password) {
        client = client.with_auth(user.to_string(), password.to_string());
    } else if user.is_some() || password.is_some() {
        bail!("--user and --password must be given together");
    }
    client = client.with_timeout(timeout).context("invalid timeout")?;

    if tls {
        #[cfg(not(feature = "tls"))]
        {
            let _ = (ca_cert, client_cert, client_key);
            bail!("TLS support requires building with --features tls");
        }
        #[cfg(feature = "tls")]
        {
            use vyna_hl7_http::tls::TlsClientConfig;

            let ca = ca_cert.context("--tls requires --ca-cert")?;
            let tls_config = match (client_cert, client_key) {
                (Some(cert), Some(key)) => TlsClientConfig::with_mtls(ca, cert, key)?,
                (None, None) => TlsClientConfig::with_ca_cert(ca)?,
                _ => bail!("mutual TLS needs both --client-cert and --client-key"),
            };
            client = client.with_tls(tls_config).context("invalid TLS config")?;
        }
    } else if ca_cert.is_some() || client_cert.is_some() || client_key.is_some() {
        bail!("--ca-cert/--client-cert/--client-key require --tls");
    }

    tokio::time::timeout(timeout, client.send_message(message))
        .await
        .context("HTTP send timed out")?
        .context("HTTP send failed")
}

fn diff_command(
    old_path: &str,
    new_path: &str,
    format: &str,
    ignore: Option<&str>,
    charset: Option<CharacterSet>,
) -> Result<()> {
    let old_text = read_input(old_path, charset)?;
    let new_text = read_input(new_path, charset)?;
    let old = parse_message(&old_text).context("Failed to parse OLD message")?;
    let new = parse_message(&new_text).context("Failed to parse NEW message")?;
    let ignore: Vec<String> = ignore
        .map(|s| s.split(',').map(|p| p.trim().to_string()).collect())
        .unwrap_or_default();
    let diffs = vyna_hl7_core::diff::diff_messages_ignoring(&old, &new, &ignore);

    match format {
        "json" => {
            let differences: Vec<_> = diffs
                .iter()
                .map(|d| {
                    json!({
                        "path": d.path,
                        "kind": match d.kind {
                            DiffKind::Added => "added",
                            DiffKind::Removed => "removed",
                            DiffKind::Changed => "changed",
                        },
                        "old": d.old,
                        "new": d.new,
                    })
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "different": !diffs.is_empty(),
                    "count": diffs.len(),
                    "differences": differences,
                }))?
            );
        }
        "text" => {
            if diffs.is_empty() {
                println!("{}", "No differences".green().bold());
            } else {
                for d in &diffs {
                    match d.kind {
                        DiffKind::Added => println!(
                            "{} {}: {}",
                            "+".green().bold(),
                            d.path.cyan(),
                            d.new.as_deref().unwrap_or("").green()
                        ),
                        DiffKind::Removed => println!(
                            "{} {}: {}",
                            "-".red().bold(),
                            d.path.cyan(),
                            d.old.as_deref().unwrap_or("").red()
                        ),
                        DiffKind::Changed => println!(
                            "{} {}: {} {} {}",
                            "~".yellow().bold(),
                            d.path.cyan(),
                            d.old.as_deref().unwrap_or("").red(),
                            "->".dimmed(),
                            d.new.as_deref().unwrap_or("").green()
                        ),
                    }
                }
            }
        }
        other => bail!("Unknown diff format '{other}' (expected text or json)"),
    }

    // diff exit convention: 1 when the messages differ.
    if !diffs.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}
