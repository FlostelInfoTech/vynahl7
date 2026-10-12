# Real-World Use Cases: From EHR to Lab Systems

*Part 11 of a 12-part series on VynaHL7, a comprehensive Rust library for HL7 v2 healthcare message processing.*

---

In the [previous post](./10-production-ready-integrations.md), we covered production patterns. Now let's explore complete real-world integration scenarios that demonstrate VynaHL7 in action.

## Use Case 1: Patient Admission Workflow (ADT)

When a patient is admitted to a hospital, multiple systems need to be notified: the EHR, billing, pharmacy, laboratory, and nursing systems.

### ADT Message Flow

```
Registration → ADT^A01 → EHR
                      → Pharmacy
                      → Laboratory
                      → Nursing Station
                      → Billing
```

### Implementation

```rust
use vyna_hl7_parser::parse_message;
use vyna_hl7_core::builders::adt::AdtBuilder;
use vyna_hl7_core::Version;
use vyna_hl7_mllp::MllpClient;
use vyna_hl7_terser::CachedTerser;

struct AdtRouter {
    ehr_client: MllpClient,
    pharmacy_client: MllpClient,
    lab_client: MllpClient,
    billing_client: MllpClient,
}

impl AdtRouter {
    async fn route_admission(&mut self, message: &Message) -> Result<(), IntegrationError> {
        let mut terser = CachedTerser::new(message);

        // Extract key patient info
        let mrn = terser.get("PID-3-1")?.ok_or("Missing MRN")?;
        let patient_class = terser.get("PV1-2")?.ok_or("Missing patient class")?;

        log::info!("Routing admission for patient {} (class: {})", mrn, patient_class);

        // Send to all downstream systems concurrently
        let (ehr_result, pharmacy_result, lab_result, billing_result) = tokio::join!(
            self.ehr_client.send_message(message),
            self.pharmacy_client.send_message(message),
            self.lab_client.send_message(message),
            self.billing_client.send_message(message)
        );

        // Check all ACKs
        verify_ack(ehr_result?, "EHR")?;
        verify_ack(pharmacy_result?, "Pharmacy")?;
        verify_ack(lab_result?, "Laboratory")?;
        verify_ack(billing_result?, "Billing")?;

        log::info!("Admission routed successfully for patient {}", mrn);
        Ok(())
    }

    async fn handle_discharge(&mut self, message: &Message) -> Result<(), IntegrationError> {
        let mut terser = CachedTerser::new(message);
        let mrn = terser.get("PID-3-1")?.ok_or("Missing MRN")?;

        log::info!("Processing discharge for patient {}", mrn);

        // Discharge notification to all systems
        let results = tokio::join!(
            self.ehr_client.send_message(message),
            self.billing_client.send_message(message)  // Generate final bill
        );

        verify_ack(results.0?, "EHR")?;
        verify_ack(results.1?, "Billing")?;

        Ok(())
    }

    async fn handle_transfer(&mut self, message: &Message) -> Result<(), IntegrationError> {
        let mut terser = CachedTerser::new(message);

        let mrn = terser.get("PID-3-1")?.ok_or("Missing MRN")?;
        let from_location = terser.get("PV1-3")?.unwrap_or("Unknown");
        let to_location = terser.get("PV1-6")?.unwrap_or("Unknown");

        log::info!("Transfer: Patient {} from {} to {}", mrn, from_location, to_location);

        // Notify relevant nursing stations
        let ack = self.ehr_client.send_message(message).await?;
        verify_ack(ack, "EHR")?;

        Ok(())
    }
}

// Main router
async fn route_adt_message(router: &mut AdtRouter, hl7: &str) -> Result<(), IntegrationError> {
    let message = parse_message(hl7)?;

    match message.get_message_type() {
        Some(("ADT", "A01")) => router.route_admission(&message).await,
        Some(("ADT", "A02")) => router.handle_transfer(&message).await,
        Some(("ADT", "A03")) => router.handle_discharge(&message).await,
        Some(("ADT", "A04")) => router.route_admission(&message).await,  // Registration
        Some(("ADT", "A08")) => router.route_admission(&message).await,  // Update
        Some((msg_type, trigger)) => {
            log::warn!("Unhandled ADT event: {}^{}", msg_type, trigger);
            Ok(())
        }
        None => Err(IntegrationError::MissingField("Message type".into())),
    }
}
```

## Use Case 2: Laboratory Results Integration (ORU)

Lab results flow from analyzers through the LIS (Laboratory Information System) to the EHR and potentially to external registries.

### ORU Message Flow

```
Analyzer → LIS → ORU^R01 → EHR
                        → Public Health Registry (reportable conditions)
                        → FHIR Server (for patient portal)
```

### Implementation

```rust
use vyna_hl7_fhir::converters::{PatientConverter, ObservationConverter};

struct LabResultProcessor {
    ehr_client: MllpClient,
    fhir_client: reqwest::Client,
    registry_client: Option<MllpClient>,
}

impl LabResultProcessor {
    async fn process_result(&mut self, message: &Message) -> Result<(), IntegrationError> {
        let mut terser = CachedTerser::new(message);

        // Extract key info
        let mrn = terser.get("PID-3-1")?.ok_or("Missing MRN")?;
        let order_id = terser.get("OBR-2")?.unwrap_or("Unknown");

        log::info!("Processing lab results for patient {} (order: {})", mrn, order_id);

        // 1. Send to EHR
        let ack = self.ehr_client.send_message(message).await?;
        verify_ack(ack, "EHR")?;

        // 2. Check for reportable conditions
        if self.is_reportable(message)? {
            if let Some(ref mut registry) = self.registry_client {
                log::info!("Sending reportable result to public health registry");
                let ack = registry.send_message(message).await?;
                verify_ack(ack, "Registry")?;
            }
        }

        // 3. Convert to FHIR and send to server (for patient portal)
        self.send_to_fhir(message).await?;

        Ok(())
    }

    fn is_reportable(&self, message: &Message) -> Result<bool, IntegrationError> {
        let terser = Terser::new(message);

        // Check each OBX for reportable conditions
        let obx_count = message.get_segments_by_id("OBX").len();

        for i in 1..=obx_count {
            let path = if i == 1 { "OBX-3-1".to_string() } else { format!("OBX({})-3-1", i) };
            if let Some(code) = terser.get(&path)? {
                // Check against reportable conditions list
                if REPORTABLE_CONDITIONS.contains(&code) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    async fn send_to_fhir(&self, message: &Message) -> Result<(), IntegrationError> {
        // Convert to FHIR resources
        let patient = PatientConverter::convert(message)?;
        let observations = ObservationConverter::convert_all(message)?;

        // Create a Bundle
        let mut entries = vec![
            serde_json::json!({
                "resource": patient,
                "request": {"method": "PUT", "url": format!("Patient/{}", patient.id.unwrap_or_default())}
            })
        ];

        for obs in observations {
            entries.push(serde_json::json!({
                "resource": obs,
                "request": {"method": "POST", "url": "Observation"}
            }));
        }

        let bundle = serde_json::json!({
            "resourceType": "Bundle",
            "type": "transaction",
            "entry": entries
        });

        // Send to FHIR server
        let response = self.fhir_client
            .post("https://fhir.example.org/fhir")
            .json(&bundle)
            .send()
            .await?;

        if !response.status().is_success() {
            log::error!("FHIR submission failed: {}", response.status());
        }

        Ok(())
    }
}

const REPORTABLE_CONDITIONS: &[&str] = &[
    "COVID19",
    "HIV",
    "HEPATITIS",
    "TB",
    // ... other reportable conditions
];
```

## Use Case 3: Pharmacy Order Integration (RDE/RDS)

Medication orders flow from the prescriber through the EHR to the pharmacy system, with dispense confirmations flowing back.

### Pharmacy Message Flow

```
Prescriber → EHR → RDE^O11 → Pharmacy
                          ←  RDS^O13 (Dispense)
                          → Medication Administration Record
```

### Implementation

```rust
use vyna_hl7_core::builders::pharmacy::RdeO11Builder;

struct PharmacyIntegration {
    pharmacy_client: MllpClient,
    mar_client: MllpClient,  // Medication Administration Record
}

impl PharmacyIntegration {
    async fn send_order(&mut self, order: MedicationOrder) -> Result<(), IntegrationError> {
        // Build RDE^O11 message
        let message = RdeO11Builder::new(Version::V2_5)
            .sending_application("EHR")
            .sending_facility("Hospital")
            .receiving_application("Pharmacy")
            .receiving_facility("Hospital")
            .patient_id(&order.patient_id)
            .patient_name(&order.patient_last, &order.patient_first)
            .order_control("NW")  // New order
            .placer_order_number(&order.order_id)
            .give_code(&order.medication_code)
            .build()?;

        let ack = self.pharmacy_client.send_message(&message).await?;
        verify_ack(ack, "Pharmacy")?;

        log::info!("Medication order {} sent to pharmacy", order.order_id);
        Ok(())
    }

    async fn process_dispense(&mut self, message: &Message) -> Result<(), IntegrationError> {
        let terser = Terser::new(message);

        // Extract dispense info
        let order_id = terser.get("ORC-2")?.ok_or("Missing order ID")?;
        let medication = terser.get("RXD-2")?.ok_or("Missing medication")?;
        let quantity = terser.get("RXD-4")?.ok_or("Missing quantity")?;

        log::info!("Dispense received: {} of {} for order {}",
            quantity, medication, order_id);

        // Forward to MAR for nursing documentation
        let ack = self.mar_client.send_message(message).await?;
        verify_ack(ack, "MAR")?;

        Ok(())
    }

    async fn cancel_order(&mut self, order_id: &str) -> Result<(), IntegrationError> {
        let message = RdeO11Builder::new(Version::V2_5)
            .sending_application("EHR")
            .sending_facility("Hospital")
            .receiving_application("Pharmacy")
            .receiving_facility("Hospital")
            .order_control("CA")  // Cancel
            .placer_order_number(order_id)
            .build()?;

        let ack = self.pharmacy_client.send_message(&message).await?;
        verify_ack(ack, "Pharmacy")?;

        log::info!("Order {} cancelled", order_id);
        Ok(())
    }
}
```

## Use Case 4: Scheduling Integration (SIU)

Appointment scheduling messages coordinate between scheduling systems, EHRs, and notification systems.

### SIU Message Flow

```
Scheduler → SIU^S12 → EHR
                   → Patient Notification
                   → Resource Management
```

### Implementation

```rust
use vyna_hl7_core::builders::siu::SiuBuilder;

struct SchedulingIntegration {
    ehr_client: MllpClient,
    notification_service: NotificationService,
}

impl SchedulingIntegration {
    async fn book_appointment(&mut self, appt: Appointment) -> Result<(), IntegrationError> {
        let message = SiuBuilder::s12(Version::V2_5)  // New appointment
            .sending_application("Scheduler")
            .sending_facility("Clinic")
            .receiving_application("EHR")
            .receiving_facility("Clinic")
            .patient_id(&appt.patient_id)
            .patient_name(&appt.patient_last, &appt.patient_first)
            .appointment_id(&appt.appointment_id)
            .start_datetime(&appt.start_time)
            .duration(appt.duration_minutes)
            .provider_id(&appt.provider_id)
            .provider_name(&appt.provider_name)
            .location(&appt.location)
            .appointment_type(&appt.type_code)
            .build()?;

        let ack = self.ehr_client.send_message(&message).await?;
        verify_ack(ack, "EHR")?;

        // Send patient notification
        self.notification_service.send_appointment_reminder(
            &appt.patient_phone,
            &appt.start_time,
            &appt.provider_name,
            &appt.location
        ).await?;

        Ok(())
    }

    async fn cancel_appointment(&mut self, appt_id: &str, reason: &str) -> Result<(), IntegrationError> {
        let message = SiuBuilder::s15(Version::V2_5)  // Cancel
            .sending_application("Scheduler")
            .sending_facility("Clinic")
            .appointment_id(appt_id)
            .cancel_reason(reason)
            .build()?;

        let ack = self.ehr_client.send_message(&message).await?;
        verify_ack(ack, "EHR")?;

        Ok(())
    }
}
```

## Use Case 5: Message Transformation Hub

Many organizations need a central hub that transforms messages between systems with different requirements.

### Hub Architecture

```
System A (v2.3) → Hub → System B (v2.5)
                    → System C (FHIR)
                    → Archive
```

### Implementation

```rust
struct TransformationHub {
    routes: HashMap<String, Vec<RouteConfig>>,
}

struct RouteConfig {
    destination: String,
    transform: Box<dyn Fn(&Message) -> Result<Message, IntegrationError> + Send + Sync>,
    client: Arc<Mutex<MllpClient>>,
}

impl TransformationHub {
    async fn process(&self, message: Message) -> Result<(), IntegrationError> {
        let (msg_type, trigger) = message.get_message_type()
            .ok_or(IntegrationError::MissingField("Message type".into()))?;

        let route_key = format!("{}^{}", msg_type, trigger);

        if let Some(routes) = self.routes.get(&route_key) {
            for route in routes {
                // Transform message for this destination
                let transformed = (route.transform)(&message)?;

                // Send to destination
                let mut client = route.client.lock().await;
                let ack = client.send_message(&transformed).await?;
                verify_ack(ack, &route.destination)?;

                log::info!("Message routed to {}", route.destination);
            }
        } else {
            log::warn!("No routes configured for {}", route_key);
        }

        Ok(())
    }
}

// Example transforms
fn upgrade_v23_to_v25(message: &Message) -> Result<Message, IntegrationError> {
    let mut new_msg = message.clone();

    // Update version in MSH-12
    if let Some(msh) = new_msg.get_mut_segment("MSH") {
        msh.set_field_value(12, "2.5")?;
    }

    // Handle field differences between versions
    // ...

    Ok(new_msg)
}

fn anonymize_for_research(message: &Message) -> Result<Message, IntegrationError> {
    let mut new_msg = message.clone();
    let mut terser = TerserMut::new(&mut new_msg);

    // Remove identifying information
    terser.set("PID-5", "RESEARCH^SUBJECT")?;
    terser.set("PID-7", "")?;  // Remove DOB
    terser.set("PID-11", "")?; // Remove address
    terser.set("PID-13", "")?; // Remove phone
    terser.set("PID-19", "")?; // Remove SSN

    Ok(new_msg)
}
```

## Use Case 6: Real-Time Clinical Decision Support

Intercept lab results to trigger clinical alerts:

```rust
struct ClinicalAlertProcessor {
    alert_service: AlertService,
}

impl ClinicalAlertProcessor {
    async fn process_result(&self, message: &Message) -> Result<(), IntegrationError> {
        let terser = Terser::new(message);

        // Get patient info
        let mrn = terser.get("PID-3-1")?.unwrap_or("Unknown");
        let patient_name = format!("{} {}",
            terser.get("PID-5-2")?.unwrap_or(""),
            terser.get("PID-5-1")?.unwrap_or("")
        );

        // Check each observation for critical values
        let obx_segments = message.get_segments_by_id("OBX");

        for (i, obx) in obx_segments.iter().enumerate() {
            let abnormal_flag = obx.get_field_value(8);

            if matches!(abnormal_flag, Some("HH") | Some("LL") | Some("AA")) {
                // Critical value detected!
                let test_name = obx.get_field(3)
                    .and_then(|f| f.get_repetition(0))
                    .and_then(|r| r.get_component(1))
                    .and_then(|c| c.value())
                    .unwrap_or("Unknown test");

                let value = obx.get_field_value(5).unwrap_or("?");
                let units = obx.get_field_value(6).unwrap_or("");

                let alert = CriticalAlert {
                    patient_mrn: mrn.to_string(),
                    patient_name: patient_name.clone(),
                    test_name: test_name.to_string(),
                    value: format!("{} {}", value, units),
                    flag: abnormal_flag.unwrap_or("").to_string(),
                    timestamp: chrono::Utc::now(),
                };

                self.alert_service.send_critical_alert(alert).await?;
            }
        }

        Ok(())
    }
}
```

## Summary

These real-world use cases demonstrate VynaHL7's versatility:

| Use Case | Message Types | Key Features |
|----------|---------------|--------------|
| Patient Admission | ADT^A01-A08 | Multi-destination routing, concurrent delivery |
| Lab Results | ORU^R01 | FHIR conversion, public health reporting |
| Pharmacy | RDE^O11, RDS^O13 | Order lifecycle management |
| Scheduling | SIU^S12-S15 | Patient notifications |
| Transformation Hub | All | Version conversion, anonymization |
| Clinical Alerts | ORU^R01 | Real-time decision support |

VynaHL7 provides the building blocks for any HL7 integration scenario, from simple point-to-point connections to complex enterprise integration hubs.

---

*Next in series: [Advanced Topics: CLI Tool, WebAssembly, and the Future](./12-advanced-topics.md)*

*Previous: [Building Production-Ready Integrations](./10-production-ready-integrations.md)*
