# VynaHL7 - HL7 v2.x Library for Rust

A comprehensive Rust library for parsing, validating, and creating HL7 v2.x healthcare messages, inspired by the Java HAPI library.

## Features

- **🖥️ GUI Test Panel**: Interactive workbench for parsing, building, validating, and testing HL7 messages
- **✅ Parsing and Serialization**: Parse HL7 pipe-delimited messages into structured data and serialize back
- **✅ Multiple HL7 Versions**: Support for HL7 v2.1 through v2.8.2 (2.1, 2.2, 2.3, 2.3.1, 2.4, 2.5, 2.5.1, 2.6, 2.7, 2.7.1, 2.8, 2.8.1, 2.8.2)
- **✅ Message Validation**: Validate messages against HL7 standards with detailed error reporting
- **✅ Schema-Based Validation**: Comprehensive schemas for all HL7 versions (2.3-2.8) with complete field and component definitions
- **✅ Data Type Validation**: Format checking for all HL7 data types (dates, times, numerics, coded values, etc.)
- **✅ Vocabulary Validation**: Code set validation against HL7 standard tables (gender, patient class, processing ID, etc.)
- **✅ Conformance Profile Validation**: Validate messages against HL7 v2 conformance profiles (XML-based)
- **✅ Advanced Validation Rules Engine**: Flexible business validation with custom rules, cross-field validation, and severity levels
- **✅ Message Orchestration**: Multi-step async workflows with retry logic and error handling
- **✅ Content-Based Routing**: Route messages to handlers based on field values and predicates
- **✅ Message Filtering**: Predicate-based filtering with AND/OR logic
- **✅ Terser API**: Easy field access using path notation (e.g., `PID-5-1`, `OBX(2)-5`)
- **✅ Encoding/Escaping**: Proper handling of HL7 escape sequences
- **✅ Message Builders**: Fluent API for creating messages (ADT A01-A13/A17/A28/A31/A34/A40, BAR, MFN, ORU, ORM, OUL, OML, RDE, RAS, RDS, RGV, RRA, RRD, SIU, MDM, DFT, QRY, QBP, RSP)
- **✅ Complex Field Builders**: Builder patterns for composite data types (XPN, XAD, XTN, CX, XCN)
- **✅ Message Types**: Support for ADT (A01-A40), SIU (S12-S15), MDM (T01-T04), DFT (P03, P11), QRY (A19, Q01-Q02), QBP (Q11/Q15/Q21/Q22), RSP (K11/K21/K22), BAR (P01-P02), Pharmacy (RDE, RAS, RDS, RGV, RRD, RRA), Laboratory (OUL, OML), MFN, ORM, ORU, ACK, and other message types
- **✅ ACK Generation**: MLLP router auto-acknowledgment (AA/AE/AR); test panel auto-ACK toggle
- **✅ MLLP Support**: Network transmission using Minimal Lower Layer Protocol (intra-organization) with TLS/mTLS security
- **✅ HTTP Transport**: HL7-over-HTTP support for inter-organization communication with TLS/mTLS security
- **✅ Testing Infrastructure**: Mock servers (MLLP & HTTP) and integration testing tools with automatic test certificate generation
- **✅ FHIR Conversion**: Convert HL7 v2 messages to FHIR R4 resources - 12 production-ready converters (Patient, Observation, Encounter, DiagnosticReport, Practitioner, AllergyIntolerance, MedicationAdministration, Condition, Procedure, Immunization, ServiceRequest, Specimen)
- **✅ Custom Z-Segments**: Type-safe framework for defining and parsing custom organization-specific Z-segments
- **🚀 Fast and Safe**: Built with Rust for performance and memory safety
- **📦 Modular Design**: Use only the components you need

## Architecture

```
vyna-hl7/
├── vyna-hl7-core          - Core data structures (Message, Segment, Field)
├── vyna-hl7-parser        - HL7 message parser (delimiter-based)
├── vyna-hl7-validator     - Message validation against HL7 standards
├── vyna-hl7-conformance   - Conformance profile validation (XML-based)
├── vyna-hl7-orchestration - Message routing, filtering, and workflow orchestration
├── vyna-hl7-terser        - Path-based field access API
├── vyna-hl7-transform     - Message transformation framework
├── vyna-hl7-templates     - Message and segment templates
├── vyna-hl7-codegen       - Schema-driven code generation for typed models
├── vyna-hl7-redact        - Safe Harbor de-identification
├── vyna-hl7-convert       - Schema-driven cross-version conversion
├── vyna-hl7-queue         - Durable store-and-forward queue
├── vyna-hl7-batch         - Batch building, lenient intake, file-drop transports
├── vyna-hl7-custom        - Type-safe custom Z-segment framework
├── vyna-hl7-mllp          - MLLP protocol for network transmission (intra-organization)
├── vyna-hl7-http          - HTTP transport for inter-organization communication
├── vyna-hl7-fhir          - HL7 v2 to FHIR R4 conversion
├── vyna-hl7-xml           - XML encoding/decoding for HL7 messages
├── vyna-hl7-wasm          - WebAssembly bindings for JavaScript/TypeScript
├── vyna-hl7-cli           - Command-line interface for message analysis
├── vyna-hl7-testpanel     - GUI test panel for interactive message testing
└── vyna-hl7-macros        - Derive macros for message types
```

## VynaHL7 Test Panel - Interactive HL7 Workbench

**The VynaHL7 Test Panel is a powerful GUI application** for working with HL7 v2.x messages. It provides an all-in-one environment for healthcare integration developers and analysts.

### Features

| Tab | Description |
|-----|-------------|
| **Parser** | Parse HL7 messages with syntax highlighting and structured tree view |
| **Builder** | Create HL7 messages using intuitive form-based builders (ADT, ORU, ORM, SIU, etc.) |
| **Terser** | Extract and modify fields using path notation (e.g., `PID-5-1`, `OBX(2)-5`) |
| **Validator** | Validate messages against HL7 schemas with detailed error reporting |
| **MLLP** | Send/receive messages via MLLP protocol with built-in server and client |
| **HTTP** | Send/receive messages over HL7-over-HTTP with auth and TLS options |
| **FTP** | Transfer HL7 files via FTP/SFTP (client and server) |
| **FHIR** | Convert HL7 v2 messages to FHIR R4 resources with live preview |
| **XML** | Convert between HL7 pipe-delimited (ER7) and XML formats |

### Quick Start

```bash
# Run directly from source
cargo run -p vyna-hl7-testpanel --release

# Or install globally
cargo install --path crates/vyna-hl7-testpanel
vyna-hl7-testpanel
```

### Sample Messages

The Test Panel includes built-in sample messages for quick testing:
- ADT A01 (Admit), A02 (Transfer), A03 (Discharge), A08 (Update)
- ORU R01 (Lab Results with multiple OBX segments)
- ORM O01 (Orders)
- SIU S12 (Scheduling)
- And more...

---

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
vyna-hl7-core = "0.11.0"
vyna-hl7-parser = "0.11.0"
vyna-hl7-terser = "0.11.0"
vyna-hl7-validator = "0.11.0"
vyna-hl7-conformance = "0.11.0"   # Optional: for conformance profile validation
vyna-hl7-orchestration = "0.11.0" # Optional: for routing, filtering, and workflows
vyna-hl7-custom = "0.11.0"        # Optional: for custom Z-segment support
vyna-hl7-mllp = "0.11.0"          # Optional: for MLLP network support with TLS/mTLS (intra-organization)
vyna-hl7-http = "0.11.0"          # Optional: for HTTP transport with TLS/mTLS (inter-organization)
vyna-hl7-fhir = "0.11.0"          # Optional: for FHIR conversion (12 converters)
```

### Parsing a Message

```rust
use vyna_hl7_parser::parse_message;
use vyna_hl7_terser::Terser;

let hl7 = r"MSH|^~\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5
PID|1|12345|67890^^^MRN|DOE^JOHN^A||19800101|M";

// Parse the message
let message = parse_message(hl7)?;

// Access fields using Terser
let terser = Terser::new(&message);
let patient_name = terser.get("PID-5-1")?;  // "DOE"
let given_name = terser.get("PID-5-2")?;    // "JOHN"
let dob = terser.get("PID-7")?;             // "19800101"
```

### Creating a Message with Builders

```rust
use vyna_hl7_core::builders::adt::AdtBuilder;
use vyna_hl7_core::Version;

// Use the fluent builder API
let message = AdtBuilder::a01(Version::V2_5)
    .sending_application("MyApp")
    .sending_facility("MyFacility")
    .receiving_application("RecApp")
    .receiving_facility("RecFacility")
    .patient_id("12345")
    .patient_name("DOE", "JOHN")
    .date_of_birth("19800101")
    .sex("M")
    .patient_class("I")
    .assigned_location("ER^101^1")
    .build()?;

// Encode to HL7
let hl7_string = message.encode();
```

### Creating a Message Manually

```rust
use vyna_hl7_core::{Message, Segment, Field, Delimiters};
use vyna_hl7_terser::TerserMut;

let mut message = Message::new();

// Build MSH segment
let mut msh = Segment::new("MSH");
let delims = Delimiters::default();
msh.add_field(Field::from_value("|"));
msh.add_field(Field::from_value("^~\\&"));
msh.set_field_value(3, "MyApp")?;
msh.set_field_value(9, "ADT^A01")?;
message.add_segment(msh);

// Use Terser to set values
let mut terser = TerserMut::new(&mut message);
terser.set("PID-5-1", "SMITH")?;
terser.set("PID-5-2", "JOHN")?;
terser.set("PID-8", "M")?;

// Encode to HL7
let hl7_string = message.encode();
```

### Validation

Validation is strict by default: every finding class is an error, including
MSH-12 version drift, so a clean result means the message complies with the
HL7 specification for the validated version. Parsing is unaffected —
validation never blocks parsing, and parse/extract/convert run without it.

```rust
use vyna_hl7_validator::Validator;
use vyna_hl7_core::Version;

// Basic validation (strict)
let validator = Validator::new(Version::V2_5);
let result = validator.validate(&message);

// Schema-based validation with data type checking (strict)
let validator = Validator::for_message_type(Version::V2_5, "ADT", "A01")?;
let result = validator.validate(&message);

if result.is_valid() {
    println!("Message is valid!");
} else {
    for error in &result.errors {
        println!("Error at {}: {}", error.location, error.message);
    }
}

// Data type validation
use vyna_hl7_core::types::DataType;
use vyna_hl7_validator::validate_data_type;

let validation = validate_data_type("20240315", DataType::DT);
if validation.is_valid() {
    println!("Valid date!");
}

// Vocabulary validation
use vyna_hl7_validator::TableRegistry;

let registry = TableRegistry::new();
let vocab_result = registry.validate("0001", "M"); // Table 0001: Administrative Sex
if vocab_result.is_valid() {
    println!("Valid gender code!");
}

// Site-local codes: extend a standard table instead of weakening the
// validator (here: NDC drug codes alongside CVX in table 0292)
let mut validator = Validator::for_message_type(Version::V2_5, "RDE", "O11")?;
validator
    .table_registry_mut()
    .get_table_mut("0292")
    .unwrap()
    .add_value("00093-0058", "Lorazepam 1mg tablet (site NDC)", false);

// Conformance profile validation
use vyna_hl7_conformance::{ProfileParser, ConformanceValidator};

let profile = ProfileParser::parse_file("profiles/adt_a01.xml")?;
let conformance_validator = ConformanceValidator::new(profile);
let conformance_result = conformance_validator.validate(&message);

if !conformance_result.is_valid() {
    for error in &conformance_result.errors {
        println!("Conformance error at {}: {}", error.location, error.message);
    }
}
```

### Advanced Validation Rules

VynaHL7 provides a flexible rules engine for implementing custom business validation:

```rust
use vyna_hl7_validator::{RulesEngine, ValidationRule, RuleSeverity, CrossFieldValidator};
use vyna_hl7_terser::Terser;

let mut engine = RulesEngine::new();

// Add a custom rule: the condition returns true when the message passes
let rule = ValidationRule::new(
    "patient_age_check",
    "Patients under 18 must have a guardian contact",
    RuleSeverity::Error,
)
.with_condition(|msg| {
    let terser = Terser::new(msg);

    // Check if patient is under 18
    if let Some(dob) = terser.get("PID-7").ok().flatten() {
        // Calculate age logic here...
        // If under 18, a guardian is required
        if let Some(guardian) = terser.get("NK1-2").ok().flatten() {
            return !guardian.is_empty(); // Pass when a guardian is present
        }
        return false; // No guardian found: violation
    }
    true // No DOB, skip rule
});

engine.add_rule(rule);

// Use pre-built cross-field validators
engine.add_rule(
    CrossFieldValidator::if_then(
        "marital_status_religion",
        "If married, religion code required",
        RuleSeverity::Warning,
        "PID-16", "M", "PID-17",
    )
);

// Validate message
let result = engine.validate(&message);

if !result.passed() {
    for violation in result.errors() {
        println!("Error: {} - {}", violation.rule_name, violation.message);
    }
    for violation in result.warnings() {
        println!("Warning: {} - {}", violation.rule_name, violation.message);
    }
}
```

**Features:**
- Custom validation rules with closures
- Severity levels (Error, Warning, Info)
- Cross-field validation patterns
- Pre-built validators for common patterns
- Location tracking for violations

### Message Orchestration and Routing

VynaHL7 provides a comprehensive orchestration framework for building message processing workflows:

```rust
use vyna_hl7_orchestration::{
    MessageOrchestrator, RetryConfig,
    ContentRouter, MessageFilter,
    OrchestrationError,
};
use vyna_hl7_terser::Terser;

// Create a filter for production messages
let mut filter = MessageFilter::new();
filter.add_rule("production_only", |msg| {
    let terser = Terser::new(msg);
    terser.get("MSH-11").ok().flatten().as_deref() == Some("P")
});

// Create a multi-step orchestration workflow
let mut orchestrator = MessageOrchestrator::new();

orchestrator
    .add_step("validate", |msg| {
        Box::pin(async move {
            // Validation logic
            Ok(msg)
        })
    })
    .add_step_with_retry(
        "enrich",
        |msg| {
            Box::pin(async move {
                // Enrichment logic with external service call
                Ok(msg)
            })
        },
        RetryConfig::standard(), // 3 attempts, 100ms delay
    )
    .add_step("transform", |msg| {
        Box::pin(async move {
            // Transformation logic
            Ok(msg)
        })
    });

// Set error handler
orchestrator.set_error_handler(|step_name, error, _msg| {
    Box::pin(async move {
        eprintln!("Error in {}: {}", step_name, error);
    })
});

// Create a content-based router
let mut router = ContentRouter::new();

router.add_route(
    "adt_handler",
    |msg| {
        let terser = Terser::new(msg);
        terser.get("MSH-9-1").ok().flatten().as_deref() == Some("ADT")
    },
    |msg| {
        Box::pin(async move {
            println!("Routing to ADT handler");
            Ok(msg)
        })
    },
);

router.set_default_handler(|msg| {
    Box::pin(async move {
        println!("Routing to default handler");
        Ok(msg)
    })
});

// Execute complete workflow
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Filter
    filter.filter(&message)?;

    // 2. Orchestrate
    let processed = orchestrator.execute(message).await?;

    // 3. Route
    router.route(processed).await?;

    Ok(())
}
```

**Features:**
- Multi-step async workflows with tokio
- Configurable retry logic with exponential backoff
- Content-based routing with predicates
- Message filtering (AND/OR logic)
- Error handling and recovery
- Fail-fast or continue-on-error execution modes

### MLLP Server

```rust
use vyna_hl7_mllp::MllpServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = MllpServer::bind("127.0.0.1:2575").await?;

    loop {
        let mut conn = server.accept().await?;

        tokio::spawn(async move {
            let message = conn.receive_message().await?;
            // Process message...
            let ack = create_ack(&message)?;
            conn.send_message(&ack).await?;
        });
    }
}
```

### MLLP Client

```rust
use vyna_hl7_mllp::MllpClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = MllpClient::connect("127.0.0.1:2575").await?;

    let ack = client.send_message(&message).await?;
    println!("Received ACK: {}", ack.get_control_id().unwrap());

    client.close().await?;
}
```

### HTTP Server

```rust
use vyna_hl7_http::HttpServer;
use vyna_hl7_core::Message;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = HttpServer::new()
        .with_handler(Arc::new(|message: Message| {
            println!("Received: {:?}", message.get_message_type());
            // Create and return ACK
            Ok(message) // Simplified for example
        }));
        // Optional: .with_auth("username".into(), "password".into());

    server.serve("127.0.0.1:8080").await?;
}
```

### HTTP Client

```rust
use vyna_hl7_http::HttpClient;
use vyna_hl7_core::builders::adt::AdtBuilder;
use vyna_hl7_core::Version;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = HttpClient::new("http://example.com/hl7")?
        .with_timeout(Duration::from_secs(30))?;
        // Optional: .with_auth("username".into(), "password".into());

    let message = AdtBuilder::a01(Version::V2_5)
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .build()?;

    let ack = client.send_message(&message).await?;
    println!("ACK received: {:?}", ack.get_control_id());
}
```

### MLLP with TLS/mTLS

```rust
use vyna_hl7_mllp::{MllpServer, MllpClient, tls::{TlsServerConfig, TlsClientConfig}};

// Server with TLS
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tls_config = TlsServerConfig::new("server-cert.pem", "server-key.pem")?;
    let server = MllpServer::bind_tls("127.0.0.1:2575", tls_config).await?;

    loop {
        let mut conn = server.accept().await?;
        tokio::spawn(async move {
            let message = conn.receive_message().await?;
            let ack = create_ack(&message)?;
            conn.send_message(&ack).await?;
        });
    }
}

// Client with TLS
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tls_config = TlsClientConfig::with_ca_cert("ca-cert.pem")?;
    let mut client = MllpClient::connect_tls("127.0.0.1:2575", "localhost", tls_config).await?;

    let ack = client.send_message(&message).await?;
    client.close().await?;
}

// Server with mTLS (client certificate verification)
let tls_config = TlsServerConfig::with_mtls(
    "server-cert.pem",
    "server-key.pem",
    "ca-cert.pem"  // CA for verifying client certificates
)?;

// Client with mTLS (provides client certificate)
let tls_config = TlsClientConfig::with_mtls(
    "ca-cert.pem",
    "client-cert.pem",
    "client-key.pem"
)?;
```

### HTTP with TLS/mTLS

```rust
use vyna_hl7_http::{HttpServer, HttpClient, tls::{TlsServerConfig, TlsClientConfig}};

// Server with TLS
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tls_config = TlsServerConfig::new("server-cert.pem", "server-key.pem")?;
    let server = HttpServer::new()
        .with_handler(handler)
        .with_tls(tls_config);

    server.serve_tls("127.0.0.1:8443").await?;
}

// Client with TLS
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tls_config = TlsClientConfig::with_ca_cert("ca-cert.pem")?;
    let client = HttpClient::new("https://example.com/hl7")?.with_tls(tls_config)?;

    let ack = client.send_message(&message).await?;
}
```

### Testing Infrastructure

```rust
use vyna_hl7_mllp::testing::MockMllpServer;
use vyna_hl7_http::testing::MockHttpServer;

// Mock MLLP server for integration tests
#[tokio::test]
async fn test_mllp_integration() {
    let server = MockMllpServer::new()
        .with_handler(|msg| {
            // Custom message handler
            Ok(create_ack(&msg))
        })
        .start()
        .await
        .unwrap();

    let mut client = MllpClient::connect(&server.url()).await.unwrap();
    let ack = client.send_message(&test_message).await.unwrap();
    // Assertions...

    server.shutdown().await.unwrap();
}

// Mock HTTP server for integration tests
#[tokio::test]
async fn test_http_integration() {
    let server = MockHttpServer::new()
        .with_auth("user".to_string(), "pass".to_string())
        .start()
        .await
        .unwrap();

    let client = HttpClient::new(&server.url())
        .unwrap()
        .with_auth("user".to_string(), "pass".to_string());

    let ack = client.send_message(&test_message).await.unwrap();
    // Assertions...

    server.shutdown().await.unwrap();
}
```

### FHIR Conversion

```rust
use vyna_hl7_fhir::prelude::*;
use vyna_hl7_parser::parse_message;

let hl7 = r"MSH|^~\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5
PID|1||67890^^^MRN||DOE^JOHN^A||19800101|M|||123 Main St^^Boston^MA^02101||555-1234
PV1||I|ER^101^1||||12345^SMITH^JANE^^^MD";

let message = parse_message(hl7)?;

// Convert to FHIR resources
let patient = PatientConverter::convert(&message)?;
let encounter = EncounterConverter::convert(&message)?;
let practitioner = PractitionerConverter::convert_attending_doctor(&message)?;

// Serialize to JSON
let json = serde_json::to_string_pretty(&patient)?;
println!("{}", json);
```

**Available Converters (12 total):**
- Patient (PID → Patient)
- Observation (OBX → Observation)
- Practitioner (PV1/ORC → Practitioner)
- Encounter (PV1 → Encounter)
- DiagnosticReport (OBR → DiagnosticReport)
- AllergyIntolerance (AL1 → AllergyIntolerance)
- MedicationAdministration (RXA → MedicationAdministration)
- Condition (PRB/DG1 → Condition)
- Procedure (PR1 → Procedure)
- **Immunization (RXA → Immunization)** ✨ NEW
- **ServiceRequest (ORC/OBR → ServiceRequest)** ✨ NEW
- **Specimen (SPM → Specimen)** ✨ NEW

See [vyna-hl7-fhir/README.md](crates/vyna-hl7-fhir/README.md) for complete documentation.

### Custom Z-Segments

VynaHL7 provides a type-safe framework for working with custom organization-specific Z-segments:

```rust
use vyna_hl7_custom::{z_segment, MessageExt};
use vyna_hl7_parser::parse_message;

// Define a custom Z-segment
z_segment! {
    ZPV,  // Patient Visit Extension
    id = "ZPV",
    fields = {
        1 => visit_type: String,
        2 => visit_number: String,
        3 => patient_class: Option<String>,
        4 => department_code: Option<String>,
    }
}

// Parse a message containing the Z-segment
let hl7 = r"MSH|^~\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5
PID|1|12345|67890^^^MRN|DOE^JOHN^A||19800101|M
ZPV|OUTPATIENT|V12345|O|CARDIO";

let message = parse_message(hl7)?;

// Extract the custom segment
if let Some(zpv) = message.get_custom_segment::<ZPV>()? {
    println!("Visit Type: {}", zpv.visit_type);
    println!("Visit Number: {}", zpv.visit_number);
}

// Build a Z-segment programmatically
let new_zpv = ZPV::builder()
    .visit_type("EMERGENCY")
    .visit_number("V99999")
    .patient_class("E")
    .build()?;

// Add to message
message.add_custom_segment(new_zpv);
```

**Features:**
- Type-safe segment definitions with compile-time validation
- Fluent builder API for ergonomic segment creation
- Custom validation hooks for business rules
- Support for primitive types, DateTime (chrono), Option<T>, Vec<T> (repeating fields), tuple types (components), and Vec<Tuple> (repeating components)
- Zero overhead for standard HL7 segments

See [vyna-hl7-custom/README.md](crates/vyna-hl7-custom/README.md) for complete documentation and examples.

### Query/Response Messages (QBP/RSP)

VynaHL7 provides comprehensive support for HL7 v2.5+ query/response protocols using QBP (Query by Parameter) and RSP (Response) messages:

```rust
use vyna_hl7_core::builders::{qbp::QbpQ22Builder, rsp::RspK22Builder};
use vyna_hl7_core::{Version, Segment, Field};
use vyna_hl7_terser::QueryResultParser;

// Create a patient search query (QBP^Q22)
let query = QbpQ22Builder::new(Version::V2_5_1)
    .sending_application("CLINREG")
    .sending_facility("WESTCLIN")
    .receiving_application("HOSPMPI")
    .receiving_facility("HOSP")
    .query_tag("987654321")
    .parameter("@PID.5.1^SMITH")   // Family name
    .parameter("@PID.5.2^JOHN")    // Given name
    .parameter("@PID.7^19850610")  // Date of birth
    .parameter("@PID.8^M")         // Sex
    .quantity_limit("50^RD")
    .build()?;

// Create a response with matching patients (RSP^K22)
let mut pid1 = Segment::new("PID");
pid1.add_field(Field::from_value("1"));
pid1.add_field(Field::from_value(""));
pid1.add_field(Field::from_value("1001^^^MPI^MR"));
pid1.add_field(Field::from_value(""));
pid1.add_field(Field::from_value("SMITH^JOHN^A"));
pid1.add_field(Field::from_value(""));
pid1.add_field(Field::from_value("19850610"));
pid1.add_field(Field::from_value("M"));

let response = RspK22Builder::new(Version::V2_5_1)
    .sending_application("HOSPMPI")
    .sending_facility("HOSP")
    .receiving_application("CLINREG")
    .receiving_facility("WESTCLIN")
    .in_response_to("Q-20231115-045")
    .query_tag("987654321")
    .query_name("Q22^Find Candidates^HL7")
    .query_response_status("OK")
    .hit_counts(247, 1, 246)  // 247 total, 1 in response, 246 remaining
    .add_segment(pid1)
    .build()?;

// Parse query results
let parser = QueryResultParser::new(&response);
let ack = parser.parse_acknowledgment()?;

println!("Status: {:?}", ack.status);  // OK
println!("Total matches: {}", ack.total_records());  // 247
println!("Has more data: {}", ack.has_more_data());  // true

// Extract patient data
for segment in parser.get_data_segments() {
    if segment.id == "PID" {
        // Process patient record
    }
}
```

**Supported Query Types:**
- **QBP^Q11/RSP^K11** - Immunization history queries (supports CDC Z44/Z34 profiles)
- **QBP^Q15/RSP^K15** - Display-oriented queries
- **QBP^Q21/RSP^K21** - Demographic queries
- **QBP^Q22/RSP^K22** - Find candidates (patient search)

**Features:**
- Type-safe query parameter construction with @ notation for field references
- Pagination support with hit counts and continuation pointers
- Query acknowledgment parsing with status codes (OK, NF, AE, AR, TM, PD)
- Automatic query tag correlation between requests and responses
- Response control for priority, quantity limits, and delivery modality
- CDC immunization query profile support (Z44, Z34)

See the `query_response.rs` example for complete demonstrations.

### WebAssembly (JavaScript/TypeScript)

VynaHL7 can be used in browsers and Node.js via WebAssembly. The package is
not published to the npm registry; build it from source with
[wasm-pack](https://rustwasm.github.io/wasm-pack/installer/):

```bash
cargo install wasm-pack
cd crates/vyna-hl7-wasm
wasm-pack build --target web --out-dir pkg      # browsers/bundlers
wasm-pack build --target nodejs --out-dir pkg-node  # Node.js
```

```javascript
import init, { parseMessage, getTerserValue } from './pkg/vyna_hl7_wasm.js';

await init();

const hl7 = `MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5
PID|1||MRN123||DOE^JOHN||19800101|M`;

const message = parseMessage(hl7);
const familyName = getTerserValue(message, "PID-5-1");
console.log(familyName); // "DOE"
```

**Features:**
- Parse and validate HL7 messages in the browser
- Full TypeScript type definitions
- Zero-copy parsing for maximum performance
- Works with all modern browsers and Node.js

See [vyna-hl7-wasm/README.md](crates/vyna-hl7-wasm/README.md) for complete documentation and examples.

### Command-Line Interface

VynaHL7 provides a powerful CLI tool for parsing, validating, and analyzing HL7 messages:

```bash
# Install the CLI
cargo install --path crates/vyna-hl7-cli

# Parse and display message structure
vynahl7 parse message.hl7 --format pretty

# Validate against HL7 standards (strict by default; --lenient relaxes)
vynahl7 validate message.hl7

# Extract specific fields using Terser paths
vynahl7 extract message.hl7 PID-5 PID-7 PID-8

# Convert to JSON
vynahl7 convert message.hl7 --to json --pretty

# Display comprehensive message info
vynahl7 info message.hl7

# Send a message and print the acknowledgment (exit 1 on NACK)
vynahl7 send message.hl7 --to mllp://127.0.0.1:2575

# Show structural differences between two messages (exit 1 when different)
vynahl7 diff old.hl7 new.hl7
```

**Commands:**
- `parse` - Parse and display HL7 message structure (text, JSON, pretty formats)
- `validate` - Validate messages against HL7 standards with detailed error reports
- `extract` - Extract field values using Terser paths (supports indexing like `OBX(1)-5`)
- `convert` - Convert to JSON or FHIR R4 format
- `info` - Display comprehensive message information and statistics
- `send` - Send a message over MLLP or HTTP and print the acknowledgment
- `diff` - Show structural differences between two messages (text, JSON formats)

**Features:**
- Colored terminal output for enhanced readability
- Support for stdin and file input
- JSON output for programmatic processing
- Batch processing capabilities
- FHIR conversion (with `--features fhir`)

See [vyna-hl7-cli/README.md](crates/vyna-hl7-cli/README.md) for complete documentation and examples.

## Examples

The `examples/` directory contains complete working examples:

- `parse_adt.rs` - Parse and analyze an ADT^A01 message
- `create_message.rs` - Build messages programmatically (manual)
- `message_builders.rs` - Build messages using the builder API
- `complex_fields.rs` - Build complex composite fields (XPN, XAD, XTN, CX, XCN)
- `schema_validation.rs` - Validate messages using schemas
- `datatype_validation.rs` - Data type format validation examples
- `enhanced_validation.rs` - Complete validation with data type checking
- `vocabulary_validation.rs` - HL7 table/code set validation examples
- `complete_validation.rs` - Full validation with data types and vocabulary
- `mllp_server.rs` - MLLP server that receives messages and sends ACKs
- `mllp_client.rs` - MLLP client that sends messages
- `http_server.rs` (vyna-hl7-http) - HTTP server that receives HL7 messages over HTTP
- `http_client.rs` (vyna-hl7-http) - HTTP client that sends HL7 messages over HTTP
- `convert_adt.rs` (vyna-hl7-fhir) - Convert ADT^A01 to FHIR Patient/Encounter
- `convert_oru.rs` (vyna-hl7-fhir) - Convert ORU^R01 to FHIR Observation/DiagnosticReport
- `routing_example.rs` (vyna-hl7-orchestration) - Content-based message routing demonstration
- `filtering_example.rs` (vyna-hl7-orchestration) - Message filtering with ALL/ANY modes
- `workflow_example.rs` (vyna-hl7-orchestration) - Complete workflow with filtering, orchestration, and routing

Run examples:

```bash
cargo run --example parse_adt
cargo run --example create_message
cargo run --example message_builders
cargo run --example complex_fields
cargo run --example schema_validation
cargo run --example datatype_validation
cargo run --example enhanced_validation
cargo run --example vocabulary_validation
cargo run --example complete_validation
cargo run --example mllp_server
cargo run --example mllp_client  # In another terminal

# HTTP transport examples
cargo run --example http_server -p vyna-hl7-http
cargo run --example http_client -p vyna-hl7-http  # In another terminal

# FHIR conversion examples
cargo run --example convert_adt -p vyna-hl7-fhir
cargo run --example convert_oru -p vyna-hl7-fhir

# Orchestration examples
cargo run --example routing_example -p vyna-hl7-orchestration
cargo run --example filtering_example -p vyna-hl7-orchestration
cargo run --example workflow_example -p vyna-hl7-orchestration
```

## Terser Path Notation

The Terser API uses a simple path notation for accessing fields:

| Path | Description |
|------|-------------|
| `PID-5` | PID segment, field 5 |
| `PID-5-1` | PID segment, field 5, component 1 |
| `PID-5-1-2` | PID segment, field 5, component 1, subcomponent 2 |
| `OBX(2)-5` | Second OBX segment (1-indexed), field 5 |
| `PID-11(1)-1` | PID segment, field 11, first repetition, component 1 |
| `PID-11(2)-1` | PID segment, field 11, second repetition, component 1 |

## HL7 Message Hierarchy

```
Message
  └─ Segment (MSH, PID, OBX, etc.)
       └─ Field (separated by |)
            └─ Repetition (separated by ~)
                 └─ Component (separated by ^)
                      └─ Subcomponent (separated by &)
```

## Supported HL7 Versions

- HL7 v2.1
- HL7 v2.2
- HL7 v2.3
- HL7 v2.3.1
- HL7 v2.4
- HL7 v2.5
- HL7 v2.5.1
- HL7 v2.6
- HL7 v2.7
- HL7 v2.7.1
- HL7 v2.8
- HL7 v2.8.1
- HL7 v2.8.2

## Supported Message Types

### ADT - Admit/Discharge/Transfer
- **A01** - Admit/Visit Notification
- **A02** - Transfer a Patient
- **A03** - Discharge/End Visit
- **A04** - Register a Patient
- **A05** - Pre-admit a Patient
- **A06** - Change Outpatient to Inpatient
- **A07** - Change Inpatient to Outpatient
- **A08** - Update Patient Information
- **A09** - Patient Departing - Tracking
- **A10** - Patient Arriving - Tracking
- **A11** - Cancel Admit/Visit Notification
- **A12** - Cancel Transfer
- **A13** - Cancel Discharge/End Visit
- **A17** - Swap Patients
- **A28** - Add Person Information
- **A31** - Update Person Information
- **A34** - Merge Patient - Patient ID Only
- **A40** - Merge Patient - Patient Identifier List

### SIU - Scheduling Information Unsolicited
- **S12** - Notification of New Appointment Booking
- **S13** - Notification of Appointment Rescheduling
- **S14** - Notification of Appointment Modification
- **S15** - Notification of Appointment Cancellation

### MDM - Medical Document Management
- **T01** - Original Document Notification
- **T02** - Original Document Notification and Content
- **T04** - Document Status Change Notification

### DFT - Detailed Financial Transaction
- **P03** - Post Detail Financial Transaction
- **P11** - Post Detail Financial Transactions - Expanded

### QRY - Query Messages
- **A19** - Patient Query
- **Q01** - Query Sent for Immediate Response
- **Q02** - Query Sent for Deferred Response

### BAR - Billing Account Record
- **P01** - Add Patient Account
- **P02** - Purge Patient Accounts

### RDE - Pharmacy/Treatment Encoded Order
- **O11** - Pharmacy/Treatment Encoded Order

### RAS - Pharmacy/Treatment Administration
- **O17** - Pharmacy/Treatment Administration

### RDS - Pharmacy/Treatment Dispense
- **O13** - Pharmacy/Treatment Dispense

### RGV - Pharmacy/Treatment Give
- **O15** - Pharmacy/Treatment Give

### RRD - Pharmacy/Treatment Dispense Information
- **O14** - Pharmacy/Treatment Dispense Information

### RRA - Pharmacy/Treatment Administration Acknowledgment
- **O18** - Pharmacy/Treatment Administration Acknowledgment

### OUL - Unsolicited Laboratory Observation
- **R21** - Unsolicited Laboratory Observation

### OML - Laboratory Order
- **O21** - Laboratory Order

### MFN - Master File Notification
- **M01** - Master File Not Otherwise Specified

### Other Message Types
- **ORM** - Order Messages (Pharmacy, Lab, etc.)
- **ORU** - Observation Results (Lab results, etc.)
- **ACK** - General Acknowledgment

## Testing

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p vyna-hl7-parser

# Run with output
cargo test -- --nocapture
```

## Performance

VynaHL7 is optimized for high-throughput message processing:

- **Zero-copy parsing** with minimal allocations
- **Cached Terser** for faster repeated field access (~1.4x measured)
- **Optimized parsers** with pre-allocation for common patterns
- **Benchmarking suite** for performance validation

```bash
# Run benchmarks
cargo bench --workspace

# Parser benchmarks only
cargo bench -p vyna-hl7-parser

# Terser benchmarks only
cargo bench -p vyna-hl7-terser
```

**Typical Performance:**
- Small messages (ADT, 3 segments): ~16 µs parse time (~60,000 msg/sec)
- Medium messages (ORU, 13 segments): ~52 µs parse time (~19,000 msg/sec)
- Terser cached access: ~60 ns (~1.4x faster than uncached)

See [docs/performance.md](docs/performance.md) for detailed optimization guide.

## Documentation

- [Requirements specification](docs/requirements.md) — numbered functional/non-functional requirements
- [Design specification](docs/design.md) — architecture, crate catalog, component designs
- [Integration guide](docs/integration.md) — install, quickstarts, transports, troubleshooting
- [SBOM](docs/sbom/) — CycloneDX supply-chain record + regeneration
- [RFCs](docs/rfcs/) — design proposals

## Contributing

Contributions are welcome. To contribute, please:

1. Create a feature branch
2. Add tests for new functionality
3. Update the living docs (`docs/`) and CHANGELOG for behavior changes
4. Ensure all tests pass (`cargo test --workspace`, `cargo clippy`, `cargo fmt --check`)
5. Submit a pull request

## Roadmap

- [x] Complete schema definitions for all HL7 versions (v2.3-2.8) with full field and component definitions ✅
- [x] Additional message type schemas (ADT A02-A40, SIU, MDM, DFT, QRY) ✅
- [x] Message builders (ADT, BAR, MFN, ORM, ORU, SIU, MDM, DFT, QRY, QBP, RSP, pharmacy, laboratory, OMP, PPR, VXU, ACK) ✅
- [x] Enhanced data type validation (format checking) ✅
- [x] Vocabulary/code set validation ✅
- [x] More message schemas (BAR, RAS, RDE, RDS, MFN) ✅
- [x] Additional pharmacy schemas (RGV, RRD, RRA) ✅
- [x] Laboratory message schemas (OUL, OML) ✅
- [x] Additional ADT builder variants (A05-A07, A09-A13, A17, A28, A31, A40) ✅
- [x] Laboratory message builders (OUL, OML) ✅
- [x] Pharmacy message builders (RDE, RAS, RDS, RGV, RRD, RRA) ✅
- [x] Complex field builder methods (XPN, XAD, XTN, CX, XCN) ✅
- [x] HL7 FHIR conversion utilities ✅ (12 forward + 4 reverse converters complete - see vyna-hl7-fhir/README.md)
- [x] Performance optimizations ✅ (Cached Terser, optimized parsers, benchmarking suite)
- [x] WebAssembly support ✅ (Full JavaScript/TypeScript bindings - see vyna-hl7-wasm/README.md)
- [x] CLI tool for message analysis ✅ (7 commands: parse, validate, extract, convert, info, send, diff - see vyna-hl7-cli/README.md)
- [x] HTTP transport support ✅ (HL7-over-HTTP for inter-organization communication - see vyna-hl7-http/README.md)
- [x] Custom Z-segment framework ✅ (Type-safe custom segment support with validation - see vyna-hl7-custom/README.md)
- [x] Conformance profile validation ✅ (XML-based conformance profiles with usage, cardinality, and length validation - see vyna-hl7-conformance crate)
- [x] Advanced validation rules engine ✅ (Flexible business validation with custom rules, cross-field validation, and severity levels - see vyna-hl7-validator crate)
- [x] Message orchestration ✅ (Multi-step async workflows with retry logic, content-based routing, and message filtering - see vyna-hl7-orchestration crate)

## Acknowledgments

Developed with AI-assisted tooling ([Claude Code](https://claude.ai/code) and [Muse Code](https://www.muse.dev/)) alongside human design, review, and testing. Inspired by the excellent [HAPI library](https://hapifhir.github.io/hapi-hl7v2/) for Java.

## License

Copyright (c) 2024-2026 Alex SHAO Fan.

Licensed under the MIT License — see the [LICENSE](LICENSE) file.

## Comparison with HAPI

| Feature | vyna-hl7 | HAPI (Java) |
|---------|-----|-------------|
| Language | Rust | Java |
| Memory Safety | Compile-time guaranteed | Runtime checked |
| Parser | Delimiter-based | Custom |
| Async I/O | Tokio | Blocking/NIO |
| Terser API | ✅ | ✅ |
| Validation | ✅ | ✅ |
| Conformance Profiles | ✅ | ✅ |
| MLLP | ✅ | ✅ |
| HTTP Transport | ✅ | ✅ |
| Message Types | ✅ | ✅ |
| HL7 FHIR | ✅ (12 converters) | ✅ |
| Rules Engine | ✅ | ❌ |
| Orchestration | ✅ | Via Camel |
| Performance | High (Rust) | Good (Java) |

**VynaHL7 Advantages:**
- 🚀 **Native async/await** with Tokio for high-performance concurrent processing
- 🛡️ **Memory safety** guaranteed at compile-time
- ⚡ **Zero-copy parsing** with minimal allocations
- 🔧 **Advanced orchestration** built-in (routing, filtering, workflows)
- 📋 **Flexible rules engine** for business validation
- 🎯 **Type safety** throughout the API

## Resources

- [HL7 International](https://www.hl7.org/)
- [HL7 v2.x Documentation](https://www.hl7.org/implement/standards/product_brief.cfm?product_id=185)
- [HAPI Documentation](https://hapifhir.github.io/hapi-hl7v2/)
