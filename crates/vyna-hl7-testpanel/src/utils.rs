//! Utility functions for the VynaHL7 Test Panel

use once_cell::sync::Lazy;
use std::collections::HashMap;
use vyna_hl7_core::{Component, Delimiters, Field, Message, Repetition};
use vyna_hl7_validator::{MessageSchema, schema_loader};

/// Display a field value without escape encoding (for UI display)
/// Joins repetitions with the repetition separator
fn display_field(field: &Field, delimiters: &Delimiters) -> String {
    field
        .repetitions
        .iter()
        .map(|r| display_repetition(r, delimiters))
        .collect::<Vec<_>>()
        .join(&delimiters.repetition_separator.to_string())
}

/// Display a repetition value without escape encoding (for UI display)
/// Joins components with the component separator
fn display_repetition(repetition: &Repetition, delimiters: &Delimiters) -> String {
    repetition
        .components
        .iter()
        .map(|c| display_component(c, delimiters))
        .collect::<Vec<_>>()
        .join(&delimiters.component_separator.to_string())
}

/// Display a component value without escape encoding (for UI display)
/// Joins subcomponents with the subcomponent separator
fn display_component(component: &Component, delimiters: &Delimiters) -> String {
    component
        .subcomponents
        .iter()
        .map(|sc| sc.value.clone())
        .collect::<Vec<_>>()
        .join(&delimiters.subcomponent_separator.to_string())
}

/// HL7 data type to field mapping for component names
/// Format: (segment_id, field_number) -> data_type
static FIELD_DATA_TYPES: Lazy<HashMap<(&'static str, usize), &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();

    // MSH fields
    m.insert(("MSH", 3), "HD"); // Sending Application
    m.insert(("MSH", 4), "HD"); // Sending Facility
    m.insert(("MSH", 5), "HD"); // Receiving Application
    m.insert(("MSH", 6), "HD"); // Receiving Facility
    m.insert(("MSH", 9), "MSG"); // Message Type
    m.insert(("MSH", 11), "PT"); // Processing ID
    m.insert(("MSH", 12), "VID"); // Version ID

    // PID fields
    m.insert(("PID", 3), "CX"); // Patient Identifier List
    m.insert(("PID", 5), "XPN"); // Patient Name
    m.insert(("PID", 6), "XPN"); // Mother's Maiden Name
    m.insert(("PID", 9), "XPN"); // Patient Alias
    m.insert(("PID", 10), "CE"); // Race
    m.insert(("PID", 11), "XAD"); // Patient Address
    m.insert(("PID", 13), "XTN"); // Phone Number - Home
    m.insert(("PID", 14), "XTN"); // Phone Number - Business
    m.insert(("PID", 15), "CE"); // Primary Language
    m.insert(("PID", 16), "CE"); // Marital Status
    m.insert(("PID", 17), "CE"); // Religion
    m.insert(("PID", 18), "CX"); // Patient Account Number
    m.insert(("PID", 22), "CE"); // Ethnic Group

    // PV1 fields
    m.insert(("PV1", 3), "PL"); // Assigned Patient Location
    m.insert(("PV1", 6), "PL"); // Prior Patient Location
    m.insert(("PV1", 7), "XCN"); // Attending Doctor
    m.insert(("PV1", 8), "XCN"); // Referring Doctor
    m.insert(("PV1", 9), "XCN"); // Consulting Doctor
    m.insert(("PV1", 17), "XCN"); // Admitting Doctor
    m.insert(("PV1", 19), "CX"); // Visit Number

    // OBR fields
    m.insert(("OBR", 2), "EI"); // Placer Order Number
    m.insert(("OBR", 3), "EI"); // Filler Order Number
    m.insert(("OBR", 4), "CE"); // Universal Service Identifier
    m.insert(("OBR", 16), "XCN"); // Ordering Provider

    // OBX fields
    m.insert(("OBX", 3), "CE"); // Observation Identifier
    m.insert(("OBX", 6), "CE"); // Units
    m.insert(("OBX", 16), "XCN"); // Responsible Observer

    // ORC fields
    m.insert(("ORC", 2), "EI"); // Placer Order Number
    m.insert(("ORC", 3), "EI"); // Filler Order Number
    m.insert(("ORC", 10), "XCN"); // Entered By
    m.insert(("ORC", 11), "XCN"); // Verified By
    m.insert(("ORC", 12), "XCN"); // Ordering Provider

    // NK1 fields
    m.insert(("NK1", 2), "XPN"); // Name
    m.insert(("NK1", 3), "CE"); // Relationship
    m.insert(("NK1", 4), "XAD"); // Address
    m.insert(("NK1", 5), "XTN"); // Phone Number
    m.insert(("NK1", 6), "XTN"); // Business Phone Number

    // IN1 fields
    m.insert(("IN1", 3), "CX"); // Insurance Company ID
    m.insert(("IN1", 4), "XON"); // Insurance Company Name
    m.insert(("IN1", 5), "XAD"); // Insurance Company Address
    m.insert(("IN1", 16), "XPN"); // Name of Insured

    // DG1 fields
    m.insert(("DG1", 3), "CE"); // Diagnosis Code

    // AL1 fields
    m.insert(("AL1", 3), "CE"); // Allergen Code/Description

    // RXA fields
    m.insert(("RXA", 5), "CE"); // Administered Code
    m.insert(("RXA", 7), "CE"); // Administered Units
    m.insert(("RXA", 10), "XCN"); // Administering Provider
    m.insert(("RXA", 17), "CE"); // Substance Manufacturer Name

    m
});

/// Component names for common HL7 data types
/// Format: (data_type, component_number) -> component_name
static COMPONENT_NAMES: Lazy<HashMap<(&'static str, usize), &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();

    // XPN - Extended Person Name
    m.insert(("XPN", 1), "Family Name");
    m.insert(("XPN", 2), "Given Name");
    m.insert(("XPN", 3), "Second/Middle Names");
    m.insert(("XPN", 4), "Suffix");
    m.insert(("XPN", 5), "Prefix");
    m.insert(("XPN", 6), "Degree");
    m.insert(("XPN", 7), "Name Type Code");
    m.insert(("XPN", 8), "Name Representation Code");
    m.insert(("XPN", 9), "Name Context");

    // XAD - Extended Address
    m.insert(("XAD", 1), "Street Address");
    m.insert(("XAD", 2), "Other Designation");
    m.insert(("XAD", 3), "City");
    m.insert(("XAD", 4), "State/Province");
    m.insert(("XAD", 5), "Zip/Postal Code");
    m.insert(("XAD", 6), "Country");
    m.insert(("XAD", 7), "Address Type");
    m.insert(("XAD", 8), "Other Geographic Designation");
    m.insert(("XAD", 9), "County/Parish Code");

    // XTN - Extended Telecommunication Number
    m.insert(("XTN", 1), "Telephone Number");
    m.insert(("XTN", 2), "Telecommunication Use Code");
    m.insert(("XTN", 3), "Telecommunication Equipment Type");
    m.insert(("XTN", 4), "Email Address");
    m.insert(("XTN", 5), "Country Code");
    m.insert(("XTN", 6), "Area/City Code");
    m.insert(("XTN", 7), "Local Number");
    m.insert(("XTN", 8), "Extension");

    // CX - Extended Composite ID with Check Digit
    m.insert(("CX", 1), "ID Number");
    m.insert(("CX", 2), "Check Digit");
    m.insert(("CX", 3), "Check Digit Scheme");
    m.insert(("CX", 4), "Assigning Authority");
    m.insert(("CX", 5), "Identifier Type Code");
    m.insert(("CX", 6), "Assigning Facility");
    m.insert(("CX", 7), "Effective Date");
    m.insert(("CX", 8), "Expiration Date");

    // XCN - Extended Composite ID Number and Name
    m.insert(("XCN", 1), "ID Number");
    m.insert(("XCN", 2), "Family Name");
    m.insert(("XCN", 3), "Given Name");
    m.insert(("XCN", 4), "Second/Middle Names");
    m.insert(("XCN", 5), "Suffix");
    m.insert(("XCN", 6), "Prefix");
    m.insert(("XCN", 7), "Degree");
    m.insert(("XCN", 8), "Source Table");
    m.insert(("XCN", 9), "Assigning Authority");
    m.insert(("XCN", 10), "Name Type Code");
    m.insert(("XCN", 11), "Identifier Check Digit");
    m.insert(("XCN", 12), "Check Digit Scheme");
    m.insert(("XCN", 13), "Identifier Type Code");

    // CE - Coded Element
    m.insert(("CE", 1), "Identifier");
    m.insert(("CE", 2), "Text");
    m.insert(("CE", 3), "Name of Coding System");
    m.insert(("CE", 4), "Alternate Identifier");
    m.insert(("CE", 5), "Alternate Text");
    m.insert(("CE", 6), "Name of Alternate Coding System");

    // CWE - Coded With Exceptions (similar to CE)
    m.insert(("CWE", 1), "Identifier");
    m.insert(("CWE", 2), "Text");
    m.insert(("CWE", 3), "Name of Coding System");
    m.insert(("CWE", 4), "Alternate Identifier");
    m.insert(("CWE", 5), "Alternate Text");
    m.insert(("CWE", 6), "Name of Alternate Coding System");

    // HD - Hierarchic Designator
    m.insert(("HD", 1), "Namespace ID");
    m.insert(("HD", 2), "Universal ID");
    m.insert(("HD", 3), "Universal ID Type");

    // EI - Entity Identifier
    m.insert(("EI", 1), "Entity Identifier");
    m.insert(("EI", 2), "Namespace ID");
    m.insert(("EI", 3), "Universal ID");
    m.insert(("EI", 4), "Universal ID Type");

    // PL - Person Location
    m.insert(("PL", 1), "Point of Care");
    m.insert(("PL", 2), "Room");
    m.insert(("PL", 3), "Bed");
    m.insert(("PL", 4), "Facility");
    m.insert(("PL", 5), "Location Status");
    m.insert(("PL", 6), "Person Location Type");
    m.insert(("PL", 7), "Building");
    m.insert(("PL", 8), "Floor");
    m.insert(("PL", 9), "Location Description");

    // MSG - Message Type
    m.insert(("MSG", 1), "Message Code");
    m.insert(("MSG", 2), "Trigger Event");
    m.insert(("MSG", 3), "Message Structure");

    // PT - Processing Type
    m.insert(("PT", 1), "Processing ID");
    m.insert(("PT", 2), "Processing Mode");

    // VID - Version Identifier
    m.insert(("VID", 1), "Version ID");
    m.insert(("VID", 2), "Internationalization Code");
    m.insert(("VID", 3), "International Version ID");

    // XON - Extended Composite Name and ID for Organizations
    m.insert(("XON", 1), "Organization Name");
    m.insert(("XON", 2), "Organization Name Type Code");
    m.insert(("XON", 3), "ID Number");
    m.insert(("XON", 4), "Check Digit");
    m.insert(("XON", 5), "Check Digit Scheme");
    m.insert(("XON", 6), "Assigning Authority");
    m.insert(("XON", 7), "Identifier Type Code");
    m.insert(("XON", 8), "Assigning Facility");
    m.insert(("XON", 9), "Name Representation Code");
    m.insert(("XON", 10), "Organization Identifier");

    m
});

/// Subcomponent names for common HL7 data types
/// Format: (data_type, subcomponent_number) -> subcomponent_name
/// These apply to composite components like HD within CX, XCN, etc.
static SUBCOMPONENT_NAMES: Lazy<HashMap<(&'static str, usize), &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();

    // HD subcomponents (used in CX.4, XCN.9, etc.)
    m.insert(("HD", 1), "Namespace ID");
    m.insert(("HD", 2), "Universal ID");
    m.insert(("HD", 3), "Universal ID Type");

    // FN - Family Name (subcomponents of XPN.1, XCN.2)
    m.insert(("FN", 1), "Surname");
    m.insert(("FN", 2), "Own Surname Prefix");
    m.insert(("FN", 3), "Own Surname");
    m.insert(("FN", 4), "Surname Prefix from Partner");
    m.insert(("FN", 5), "Surname from Partner");

    // SAD - Street Address (subcomponent of XAD.1)
    m.insert(("SAD", 1), "Street or Mailing Address");
    m.insert(("SAD", 2), "Street Name");
    m.insert(("SAD", 3), "Dwelling Number");

    m
});

/// Get the data type for a field
pub fn get_field_data_type(segment_id: &str, field_num: usize) -> Option<&'static str> {
    FIELD_DATA_TYPES.get(&(segment_id, field_num)).copied()
}

/// Get the component name for a data type and component number
pub fn get_component_name(data_type: &str, comp_num: usize) -> Option<&'static str> {
    COMPONENT_NAMES.get(&(data_type, comp_num)).copied()
}

/// Get the subcomponent name (for HD subcomponents commonly used)
pub fn get_subcomponent_name(comp_num: usize) -> Option<&'static str> {
    // Most subcomponents follow HD pattern
    SUBCOMPONENT_NAMES.get(&("HD", comp_num)).copied()
}

/// HL7 field names mapping for common segments
/// Format: (segment_id, field_number) -> field_name
static FIELD_NAMES: Lazy<HashMap<(&'static str, usize), &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();

    // MSH - Message Header
    m.insert(("MSH", 1), "Field Separator");
    m.insert(("MSH", 2), "Encoding Characters");
    m.insert(("MSH", 3), "Sending Application");
    m.insert(("MSH", 4), "Sending Facility");
    m.insert(("MSH", 5), "Receiving Application");
    m.insert(("MSH", 6), "Receiving Facility");
    m.insert(("MSH", 7), "Date/Time of Message");
    m.insert(("MSH", 8), "Security");
    m.insert(("MSH", 9), "Message Type");
    m.insert(("MSH", 10), "Message Control ID");
    m.insert(("MSH", 11), "Processing ID");
    m.insert(("MSH", 12), "Version ID");
    m.insert(("MSH", 13), "Sequence Number");
    m.insert(("MSH", 14), "Continuation Pointer");
    m.insert(("MSH", 15), "Accept Acknowledgment Type");
    m.insert(("MSH", 16), "Application Acknowledgment Type");
    m.insert(("MSH", 17), "Country Code");
    m.insert(("MSH", 18), "Character Set");
    m.insert(("MSH", 19), "Principal Language");
    m.insert(("MSH", 20), "Alternate Character Set Handling");
    m.insert(("MSH", 21), "Message Profile Identifier");

    // PID - Patient Identification
    m.insert(("PID", 1), "Set ID");
    m.insert(("PID", 2), "Patient ID (External)");
    m.insert(("PID", 3), "Patient Identifier List");
    m.insert(("PID", 4), "Alternate Patient ID");
    m.insert(("PID", 5), "Patient Name");
    m.insert(("PID", 6), "Mother's Maiden Name");
    m.insert(("PID", 7), "Date/Time of Birth");
    m.insert(("PID", 8), "Administrative Sex");
    m.insert(("PID", 9), "Patient Alias");
    m.insert(("PID", 10), "Race");
    m.insert(("PID", 11), "Patient Address");
    m.insert(("PID", 12), "County Code");
    m.insert(("PID", 13), "Phone Number - Home");
    m.insert(("PID", 14), "Phone Number - Business");
    m.insert(("PID", 15), "Primary Language");
    m.insert(("PID", 16), "Marital Status");
    m.insert(("PID", 17), "Religion");
    m.insert(("PID", 18), "Patient Account Number");
    m.insert(("PID", 19), "SSN Number");
    m.insert(("PID", 20), "Driver's License Number");
    m.insert(("PID", 21), "Mother's Identifier");
    m.insert(("PID", 22), "Ethnic Group");
    m.insert(("PID", 23), "Birth Place");
    m.insert(("PID", 24), "Multiple Birth Indicator");
    m.insert(("PID", 25), "Birth Order");
    m.insert(("PID", 26), "Citizenship");
    m.insert(("PID", 27), "Veterans Military Status");
    m.insert(("PID", 28), "Nationality");
    m.insert(("PID", 29), "Patient Death Date/Time");
    m.insert(("PID", 30), "Patient Death Indicator");

    // PV1 - Patient Visit
    m.insert(("PV1", 1), "Set ID");
    m.insert(("PV1", 2), "Patient Class");
    m.insert(("PV1", 3), "Assigned Patient Location");
    m.insert(("PV1", 4), "Admission Type");
    m.insert(("PV1", 5), "Preadmit Number");
    m.insert(("PV1", 6), "Prior Patient Location");
    m.insert(("PV1", 7), "Attending Doctor");
    m.insert(("PV1", 8), "Referring Doctor");
    m.insert(("PV1", 9), "Consulting Doctor");
    m.insert(("PV1", 10), "Hospital Service");
    m.insert(("PV1", 11), "Temporary Location");
    m.insert(("PV1", 12), "Preadmit Test Indicator");
    m.insert(("PV1", 13), "Re-admission Indicator");
    m.insert(("PV1", 14), "Admit Source");
    m.insert(("PV1", 15), "Ambulatory Status");
    m.insert(("PV1", 16), "VIP Indicator");
    m.insert(("PV1", 17), "Admitting Doctor");
    m.insert(("PV1", 18), "Patient Type");
    m.insert(("PV1", 19), "Visit Number");
    m.insert(("PV1", 20), "Financial Class");
    m.insert(("PV1", 44), "Admit Date/Time");
    m.insert(("PV1", 45), "Discharge Date/Time");

    // EVN - Event Type
    m.insert(("EVN", 1), "Event Type Code");
    m.insert(("EVN", 2), "Recorded Date/Time");
    m.insert(("EVN", 3), "Date/Time Planned Event");
    m.insert(("EVN", 4), "Event Reason Code");
    m.insert(("EVN", 5), "Operator ID");
    m.insert(("EVN", 6), "Event Occurred");

    // OBR - Observation Request
    m.insert(("OBR", 1), "Set ID");
    m.insert(("OBR", 2), "Placer Order Number");
    m.insert(("OBR", 3), "Filler Order Number");
    m.insert(("OBR", 4), "Universal Service Identifier");
    m.insert(("OBR", 5), "Priority");
    m.insert(("OBR", 6), "Requested Date/Time");
    m.insert(("OBR", 7), "Observation Date/Time");
    m.insert(("OBR", 8), "Observation End Date/Time");
    m.insert(("OBR", 9), "Collection Volume");
    m.insert(("OBR", 10), "Collector Identifier");
    m.insert(("OBR", 11), "Specimen Action Code");
    m.insert(("OBR", 12), "Danger Code");
    m.insert(("OBR", 13), "Relevant Clinical Information");
    m.insert(("OBR", 14), "Specimen Received Date/Time");
    m.insert(("OBR", 15), "Specimen Source");
    m.insert(("OBR", 16), "Ordering Provider");
    m.insert(("OBR", 22), "Results Rpt/Status Chng Date/Time");
    m.insert(("OBR", 25), "Result Status");

    // OBX - Observation/Result
    m.insert(("OBX", 1), "Set ID");
    m.insert(("OBX", 2), "Value Type");
    m.insert(("OBX", 3), "Observation Identifier");
    m.insert(("OBX", 4), "Observation Sub-ID");
    m.insert(("OBX", 5), "Observation Value");
    m.insert(("OBX", 6), "Units");
    m.insert(("OBX", 7), "References Range");
    m.insert(("OBX", 8), "Abnormal Flags");
    m.insert(("OBX", 9), "Probability");
    m.insert(("OBX", 10), "Nature of Abnormal Test");
    m.insert(("OBX", 11), "Observation Result Status");
    m.insert(("OBX", 12), "Effective Date of Reference Range");
    m.insert(("OBX", 13), "User Defined Access Checks");
    m.insert(("OBX", 14), "Date/Time of Observation");
    m.insert(("OBX", 15), "Producer's ID");
    m.insert(("OBX", 16), "Responsible Observer");
    m.insert(("OBX", 17), "Observation Method");

    // ORC - Common Order
    m.insert(("ORC", 1), "Order Control");
    m.insert(("ORC", 2), "Placer Order Number");
    m.insert(("ORC", 3), "Filler Order Number");
    m.insert(("ORC", 4), "Placer Group Number");
    m.insert(("ORC", 5), "Order Status");
    m.insert(("ORC", 6), "Response Flag");
    m.insert(("ORC", 7), "Quantity/Timing");
    m.insert(("ORC", 8), "Parent Order");
    m.insert(("ORC", 9), "Date/Time of Transaction");
    m.insert(("ORC", 10), "Entered By");
    m.insert(("ORC", 11), "Verified By");
    m.insert(("ORC", 12), "Ordering Provider");
    m.insert(("ORC", 13), "Enterer's Location");
    m.insert(("ORC", 14), "Call Back Phone Number");
    m.insert(("ORC", 15), "Order Effective Date/Time");
    m.insert(("ORC", 16), "Order Control Code Reason");

    // NK1 - Next of Kin
    m.insert(("NK1", 1), "Set ID");
    m.insert(("NK1", 2), "Name");
    m.insert(("NK1", 3), "Relationship");
    m.insert(("NK1", 4), "Address");
    m.insert(("NK1", 5), "Phone Number");
    m.insert(("NK1", 6), "Business Phone Number");
    m.insert(("NK1", 7), "Contact Role");

    // IN1 - Insurance
    m.insert(("IN1", 1), "Set ID");
    m.insert(("IN1", 2), "Insurance Plan ID");
    m.insert(("IN1", 3), "Insurance Company ID");
    m.insert(("IN1", 4), "Insurance Company Name");
    m.insert(("IN1", 5), "Insurance Company Address");
    m.insert(("IN1", 6), "Insurance Co Contact Person");
    m.insert(("IN1", 7), "Insurance Co Phone Number");
    m.insert(("IN1", 8), "Group Number");
    m.insert(("IN1", 9), "Group Name");
    m.insert(("IN1", 10), "Insured's Group Emp ID");
    m.insert(("IN1", 11), "Insured's Group Emp Name");
    m.insert(("IN1", 12), "Plan Effective Date");
    m.insert(("IN1", 13), "Plan Expiration Date");
    m.insert(("IN1", 16), "Name of Insured");
    m.insert(("IN1", 17), "Insured's Relationship to Patient");
    m.insert(("IN1", 18), "Insured's Date of Birth");

    // DG1 - Diagnosis
    m.insert(("DG1", 1), "Set ID");
    m.insert(("DG1", 2), "Diagnosis Coding Method");
    m.insert(("DG1", 3), "Diagnosis Code");
    m.insert(("DG1", 4), "Diagnosis Description");
    m.insert(("DG1", 5), "Diagnosis Date/Time");
    m.insert(("DG1", 6), "Diagnosis Type");

    // AL1 - Allergy Information
    m.insert(("AL1", 1), "Set ID");
    m.insert(("AL1", 2), "Allergen Type Code");
    m.insert(("AL1", 3), "Allergen Code/Description");
    m.insert(("AL1", 4), "Allergy Severity Code");
    m.insert(("AL1", 5), "Allergy Reaction Code");
    m.insert(("AL1", 6), "Identification Date");

    // SCH - Scheduling Activity Information
    m.insert(("SCH", 1), "Placer Appointment ID");
    m.insert(("SCH", 2), "Filler Appointment ID");
    m.insert(("SCH", 3), "Occurrence Number");
    m.insert(("SCH", 4), "Placer Group Number");
    m.insert(("SCH", 5), "Schedule ID");
    m.insert(("SCH", 6), "Event Reason");
    m.insert(("SCH", 7), "Appointment Reason");
    m.insert(("SCH", 8), "Appointment Type");
    m.insert(("SCH", 9), "Appointment Duration");
    m.insert(("SCH", 10), "Appointment Duration Units");
    m.insert(("SCH", 11), "Appointment Timing Quantity");
    m.insert(("SCH", 25), "Filler Status Code");

    // RXA - Pharmacy/Treatment Administration
    m.insert(("RXA", 1), "Give Sub-ID Counter");
    m.insert(("RXA", 2), "Administration Sub-ID Counter");
    m.insert(("RXA", 3), "Date/Time Start of Administration");
    m.insert(("RXA", 4), "Date/Time End of Administration");
    m.insert(("RXA", 5), "Administered Code");
    m.insert(("RXA", 6), "Administered Amount");
    m.insert(("RXA", 7), "Administered Units");
    m.insert(("RXA", 8), "Administered Dosage Form");
    m.insert(("RXA", 9), "Administration Notes");
    m.insert(("RXA", 10), "Administering Provider");
    m.insert(("RXA", 15), "Substance Lot Number");
    m.insert(("RXA", 16), "Substance Expiration Date");
    m.insert(("RXA", 17), "Substance Manufacturer Name");

    // ACK fields (MSA segment)
    m.insert(("MSA", 1), "Acknowledgment Code");
    m.insert(("MSA", 2), "Message Control ID");
    m.insert(("MSA", 3), "Text Message");
    m.insert(("MSA", 4), "Expected Sequence Number");
    m.insert(("MSA", 5), "Delayed Acknowledgment Type");
    m.insert(("MSA", 6), "Error Condition");

    // ERR - Error
    m.insert(("ERR", 1), "Error Code and Location");
    m.insert(("ERR", 2), "Error Location");
    m.insert(("ERR", 3), "HL7 Error Code");
    m.insert(("ERR", 4), "Severity");
    m.insert(("ERR", 5), "Application Error Code");
    m.insert(("ERR", 6), "Application Error Parameter");
    m.insert(("ERR", 7), "Diagnostic Information");
    m.insert(("ERR", 8), "User Message");

    m
});

/// Get the field name for a segment and field number
pub fn get_field_name(segment_id: &str, field_num: usize) -> Option<&'static str> {
    FIELD_NAMES.get(&(segment_id, field_num)).copied()
}

/// Format a message as a tree structure for display
/// Try to load the schema for a parsed message
pub fn load_message_schema(message: &Message) -> Option<MessageSchema> {
    let version = message
        .get_version()
        .unwrap_or(vyna_hl7_core::Version::V2_5);
    message.get_message_type().and_then(|(msg_type, trigger)| {
        schema_loader::load_schema(version, &msg_type, &trigger).ok()
    })
}

pub fn format_message_tree(message: &Message) -> Vec<TreeNode> {
    let schema = load_message_schema(message);
    format_message_tree_with_schema(message, schema.as_ref())
}

pub fn format_message_tree_with_schema(
    message: &Message,
    schema: Option<&MessageSchema>,
) -> Vec<TreeNode> {
    let mut nodes = Vec::new();
    let delimiters = &message.delimiters;

    // Track segment counts for Terser indexing (e.g., OBX(1), OBX(2))
    let mut segment_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for (seg_idx, segment) in message.segments.iter().enumerate() {
        // Increment segment count for this type
        let seg_count = segment_counts.entry(segment.id.clone()).or_insert(0);
        *seg_count += 1;
        let seg_instance = *seg_count;

        // Get segment definition from schema
        let seg_def = schema.and_then(|s| s.segments.get(&segment.id));

        let seg_label = match seg_def {
            Some(def) => format!("{} - {} (Segment {})", segment.id, def.name, seg_idx + 1),
            None => format!("{} (Segment {})", segment.id, seg_idx + 1),
        };

        let mut segment_node = TreeNode {
            label: seg_label,
            children: Vec::new(),
            expanded: seg_idx == 0, // Expand first segment by default
            path: None,             // Segments don't have a direct Terser path
        };

        for (field_idx, field) in segment.fields.iter().enumerate() {
            // Use display function to get the full field value without escape encoding
            let field_value = display_field(field, delimiters);
            if !field_value.is_empty() {
                // Calculate the correct HL7 field number
                // VynaHL7 parser stores all fields starting at index 0 = field 1
                // For MSH: fields[0] = MSH-1 ("|"), fields[1] = MSH-2 ("^~\&"), etc.
                // For other segments: fields[0] = SEG-1, fields[1] = SEG-2, etc.
                let field_num = field_idx + 1;

                // Get field definition from schema first, fall back to hardcoded names
                let schema_field_def = seg_def.and_then(|d| d.fields.get(&field_num));
                let field_name: Option<String> = schema_field_def
                    .map(|fd| fd.name.clone())
                    .or_else(|| get_field_name(&segment.id, field_num).map(|s| s.to_string()));

                let field_label = if let Some(ref name) = field_name {
                    format!(
                        "{}-{} ({}): {}",
                        segment.id,
                        field_num,
                        name,
                        truncate_string(&field_value, 50)
                    )
                } else {
                    format!(
                        "{}-{}: {}",
                        segment.id,
                        field_num,
                        truncate_string(&field_value, 60)
                    )
                };

                // Build Terser path for this field: number the
                // segment instance only when the type repeats
                // (e.g., OBX(1)-5, OBX(2)-5); singletons stay bare).
                let total_of_this_type = message
                    .segments
                    .iter()
                    .filter(|s| s.id == segment.id)
                    .count();
                let field_path = if total_of_this_type > 1 {
                    format!("{}({})-{}", segment.id, seg_instance, field_num)
                } else {
                    format!("{}-{}", segment.id, field_num)
                };

                let mut field_node = TreeNode {
                    label: field_label,
                    children: Vec::new(),
                    expanded: false,
                    path: Some(field_path.clone()),
                };

                // Get component definitions from schema, or fall back to data type lookup
                let schema_components = schema_field_def.and_then(|fd| fd.components.as_ref());
                let data_type = schema_field_def
                    .map(|fd| fd.data_type.as_str())
                    .or_else(|| get_field_data_type(&segment.id, field_num));

                // Add repetitions if present
                for (rep_idx, repetition) in field.repetitions.iter().enumerate() {
                    if field.repetitions.len() > 1 {
                        // Use display function to get the full repetition value without escape encoding
                        let rep_value = display_repetition(repetition, delimiters);
                        if !rep_value.is_empty() {
                            // Terser paths use 1-based repetition indexes.
                            let rep_path = format!("{}({})", field_path, rep_idx + 1);
                            let rep_label =
                                format!("Rep {}: {}", rep_idx + 1, truncate_string(&rep_value, 50));

                            let mut rep_node = TreeNode {
                                label: rep_label,
                                children: Vec::new(),
                                expanded: false,
                                path: Some(rep_path.clone()),
                            };

                            // Add components with names
                            add_component_nodes(
                                &mut rep_node,
                                repetition,
                                &rep_path,
                                data_type,
                                schema_components,
                                delimiters,
                            );
                            field_node.children.push(rep_node);
                        }
                    } else {
                        // Single repetition - add components directly
                        add_component_nodes(
                            &mut field_node,
                            repetition,
                            &field_path,
                            data_type,
                            schema_components,
                            delimiters,
                        );
                    }
                }

                segment_node.children.push(field_node);
            }
        }

        nodes.push(segment_node);
    }

    nodes
}

fn add_component_nodes(
    parent: &mut TreeNode,
    repetition: &vyna_hl7_core::Repetition,
    base_path: &str,
    data_type: Option<&str>,
    schema_components: Option<&HashMap<String, vyna_hl7_validator::ComponentDefinition>>,
    delimiters: &vyna_hl7_core::Delimiters,
) {
    if repetition.components.len() > 1 {
        for (comp_idx, component) in repetition.components.iter().enumerate() {
            // Use display function to get the full component value without escape encoding
            let comp_value = display_component(component, delimiters);
            if !comp_value.is_empty() {
                let comp_num = comp_idx + 1;

                // Build component path (e.g., PID-5-1, OBX(1)-5-1)
                let comp_path = format!("{}-{}", base_path, comp_num);

                // Get component name: schema first, then hardcoded fallback
                let comp_num_str = comp_num.to_string();
                let comp_name: Option<String> = schema_components
                    .and_then(|comps| comps.get(&comp_num_str).map(|c| c.name.clone()))
                    .or_else(|| {
                        data_type
                            .and_then(|dt| get_component_name(dt, comp_num))
                            .map(|s| s.to_string())
                    });

                // Use Terser notation: SEG-F-C (e.g., MSH-9-1)
                let comp_label = if let Some(ref name) = comp_name {
                    format!(
                        "{} ({}): {}",
                        comp_path,
                        name,
                        truncate_string(&comp_value, 35)
                    )
                } else {
                    format!("{}: {}", comp_path, truncate_string(&comp_value, 40))
                };

                let mut comp_node = TreeNode {
                    label: comp_label,
                    children: Vec::new(),
                    expanded: false,
                    path: Some(comp_path.clone()),
                };

                // Add subcomponents if present
                if component.subcomponents.len() > 1 {
                    for (sub_idx, subcomp) in component.subcomponents.iter().enumerate() {
                        if !subcomp.value.is_empty() {
                            let sub_num = sub_idx + 1;

                            // Build subcomponent path (e.g., PID-5-1-1)
                            let sub_path = format!("{}-{}", comp_path, sub_num);

                            // Get subcomponent name (typically HD pattern for nested structures)
                            let sub_name = get_subcomponent_name(sub_num);

                            // Use Terser notation: SEG-F-C-S (e.g., MSH-9-1-1)
                            let sub_label = if let Some(name) = sub_name {
                                format!("{} ({}): {}", sub_path, name, subcomp.value)
                            } else {
                                format!("{}: {}", sub_path, subcomp.value)
                            };

                            comp_node.children.push(TreeNode {
                                label: sub_label,
                                children: Vec::new(),
                                expanded: false,
                                path: Some(sub_path),
                            });
                        }
                    }
                }

                parent.children.push(comp_node);
            }
        }
    }
}

/// A tree node for hierarchical display
#[derive(Clone)]
pub struct TreeNode {
    pub label: String,
    pub children: Vec<TreeNode>,
    pub expanded: bool,
    /// Optional Terser path for this node (e.g., "PID-5-1")
    pub path: Option<String>,
}

impl TreeNode {
    /// Render this tree node in the UI
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.ui_interactive(ui);
    }

    /// Render this tree node in the UI and return the path if clicked
    pub fn ui_interactive(&mut self, ui: &mut egui::Ui) -> Option<String> {
        let mut clicked_path = None;

        if self.children.is_empty() {
            // Leaf node - make it clickable if it has a path
            if self.path.is_some() {
                let response = ui.selectable_label(false, &self.label);
                if response.clicked() {
                    clicked_path = self.path.clone();
                }
                if let Some(ref path) = self.path {
                    response.on_hover_text(format!("Click to query: {}", path));
                }
            } else {
                ui.label(&self.label);
            }
        } else {
            // Parent node with children
            egui::CollapsingHeader::new(&self.label)
                .default_open(self.expanded)
                .show(ui, |ui| {
                    // If this node has a path, add a clickable button
                    if let Some(ref path) = self.path {
                        if ui
                            .small_button(format!("📋 {}", path))
                            .on_hover_text("Click to query this path")
                            .clicked()
                        {
                            clicked_path = Some(path.clone());
                        }
                        ui.add_space(2.0);
                    }

                    for child in &mut self.children {
                        if let Some(path) = child.ui_interactive(ui) {
                            clicked_path = Some(path);
                        }
                    }
                });
        }

        clicked_path
    }
}

/// Truncate a string with ellipsis if too long
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        s.chars().take(max_len).collect()
    } else {
        let mut end = max_len - 3;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &s[..end])
    }
}

/// Format bytes as human-readable size
pub fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Get message statistics
pub fn get_message_stats(message: &Message) -> MessageStats {
    let segment_count = message.segments.len();
    let mut field_count = 0;
    let mut segment_types: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for segment in &message.segments {
        *segment_types.entry(segment.id.clone()).or_insert(0) += 1;
        field_count += segment.fields.len();
    }

    let message_type = message
        .get_message_type()
        .map(|(t, e)| format!("{}^{}", t, e))
        .unwrap_or_else(|| "Unknown".to_string());

    let version = message
        .get_version()
        .map(|v| v.as_str().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    MessageStats {
        message_type,
        version,
        segment_count,
        field_count,
        segment_types,
    }
}

/// Statistics about a parsed message
pub struct MessageStats {
    pub message_type: String,
    pub version: String,
    pub segment_count: usize,
    pub field_count: usize,
    pub segment_types: std::collections::HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_string_keeps_char_boundary() {
        // CJK chars are 3 bytes each; a byte cut at max_len - 3
        // can split one and panic on non-char-boundary slicing.
        let s = "日本語テストメッセージの切り詰め確認用文字列です";
        let out = truncate_string(s, 20);
        assert!(out.ends_with("..."));
        assert!(out.len() <= 20);
        // Short strings pass through untouched.
        assert_eq!(truncate_string("abc", 20), "abc");
    }

    #[test]
    fn tree_repetition_paths_are_one_based() {
        // TerserPath rejects repetition index 0, so tree rep
        // paths must use 1-based indexes to stay clickable.
        let hl7 = "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5\r\
             PID|1||7||DOE^JOHN||||||123 Main St^^Boston^MA~456 Oak Ave^^Austin^TX\r";
        let msg = vyna_hl7_parser::parse_message(hl7).unwrap();
        let nodes = format_message_tree(&msg);
        let mut rep_paths = Vec::new();
        collect_paths(&nodes, &mut rep_paths);
        let rep_paths: Vec<_> = rep_paths
            .into_iter()
            .filter(|p| p.starts_with("PID-11(") && p.ends_with(')'))
            .collect();
        assert_eq!(rep_paths, vec!["PID-11(1)", "PID-11(2)"]);
        // Every emitted rep path must parse in the terser.
        let terser = vyna_hl7_terser::Terser::new(&msg);
        for path in &rep_paths {
            assert!(terser.get(path).is_ok(), "terser rejects {path}");
        }
    }

    #[test]
    fn tree_segment_instance_paths_number_repeats_only() {
        let hl7 = "MSH|^~\\&|S|F|R|D|20240101||ORU^R01|M1|P|2.5\r\
             PID|1||7||DOE^JOHN\r\
             OBX|1|ST|WBC||5.2\r\
             OBX|2|ST|RBC||4.1\r";
        let msg = vyna_hl7_parser::parse_message(hl7).unwrap();
        let nodes = format_message_tree(&msg);
        let mut paths = Vec::new();
        collect_paths(&nodes, &mut paths);
        assert!(paths.contains(&"PID-5".to_string()));
        assert!(paths.contains(&"OBX(1)-5".to_string()));
        assert!(paths.contains(&"OBX(2)-5".to_string()));
        assert!(!paths.iter().any(|p| p == "OBX-5"));
        let terser = vyna_hl7_terser::Terser::new(&msg);
        for path in ["PID-5", "OBX(1)-5", "OBX(2)-5"] {
            assert!(terser.get(path).is_ok(), "terser rejects {path}");
        }
    }

    fn collect_paths(nodes: &[TreeNode], out: &mut Vec<String>) {
        for node in nodes {
            if let Some(path) = &node.path {
                out.push(path.clone());
            }
            collect_paths(&node.children, out);
        }
    }
}
