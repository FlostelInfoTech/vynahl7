//! Vocabulary and code set validation for HL7 fields
//!
//! This module provides validation against HL7 standard tables and code sets.
//! HL7 defines numerous tables for coded values like gender, admission type,
//! patient class, etc.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of vocabulary validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VocabularyValidation {
    Valid,
    Invalid { reason: String },
    NotApplicable, // Field doesn't require vocabulary validation
}

impl VocabularyValidation {
    pub fn is_valid(&self) -> bool {
        matches!(self, VocabularyValidation::Valid | VocabularyValidation::NotApplicable)
    }

    pub fn error_message(&self) -> Option<&str> {
        match self {
            VocabularyValidation::Invalid { reason } => Some(reason),
            _ => None,
        }
    }
}

/// HL7 Table definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hl7Table {
    pub table_id: String,
    pub name: String,
    pub description: String,
    pub values: HashMap<String, TableValue>,
}

/// A value within an HL7 table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableValue {
    pub code: String,
    pub description: String,
    pub deprecated: bool,
}

impl Hl7Table {
    /// Create a new HL7 table
    pub fn new(table_id: &str, name: &str, description: &str) -> Self {
        Self {
            table_id: table_id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            values: HashMap::new(),
        }
    }

    /// Add a value to the table
    pub fn add_value(&mut self, code: &str, description: &str, deprecated: bool) {
        self.values.insert(
            code.to_string(),
            TableValue {
                code: code.to_string(),
                description: description.to_string(),
                deprecated,
            },
        );
    }

    /// Validate a code against this table
    pub fn validate(&self, code: &str) -> VocabularyValidation {
        if code.is_empty() {
            return VocabularyValidation::Valid;
        }

        match self.values.get(code) {
            Some(value) => {
                if value.deprecated {
                    VocabularyValidation::Invalid {
                        reason: format!(
                            "Code '{}' is deprecated in table {} ({})",
                            code, self.table_id, self.name
                        ),
                    }
                } else {
                    VocabularyValidation::Valid
                }
            }
            None => VocabularyValidation::Invalid {
                reason: format!(
                    "Invalid code '{}' for table {} ({}). Valid codes: {}",
                    code,
                    self.table_id,
                    self.name,
                    self.list_valid_codes()
                ),
            },
        }
    }

    /// List valid (non-deprecated) codes
    fn list_valid_codes(&self) -> String {
        let mut codes: Vec<String> = self
            .values
            .values()
            .filter(|v| !v.deprecated)
            .map(|v| v.code.clone())
            .collect();
        codes.sort();
        if codes.len() > 10 {
            format!("{}, ... ({} total)", codes[..10].join(", "), codes.len())
        } else {
            codes.join(", ")
        }
    }
}

/// HL7 Table Registry - manages all HL7 tables
pub struct TableRegistry {
    tables: HashMap<String, Hl7Table>,
}

impl TableRegistry {
    /// Create a new registry with standard HL7 tables
    pub fn new() -> Self {
        let mut registry = Self {
            tables: HashMap::new(),
        };
        registry.load_standard_tables();
        registry
    }

    /// Get a table by ID
    pub fn get_table(&self, table_id: &str) -> Option<&Hl7Table> {
        self.tables.get(table_id)
    }

    /// Add a custom table
    pub fn add_table(&mut self, table: Hl7Table) {
        self.tables.insert(table.table_id.clone(), table);
    }

    /// Validate a code against a specific table
    pub fn validate(&self, table_id: &str, code: &str) -> VocabularyValidation {
        match self.get_table(table_id) {
            Some(table) => table.validate(code),
            None => VocabularyValidation::NotApplicable,
        }
    }

    /// Load standard HL7 tables
    fn load_standard_tables(&mut self) {
        // HL7 Table 0001 - Administrative Sex
        let mut table_0001 = Hl7Table::new("0001", "Administrative Sex", "Patient's administrative sex");
        table_0001.add_value("F", "Female", false);
        table_0001.add_value("M", "Male", false);
        table_0001.add_value("O", "Other", false);
        table_0001.add_value("U", "Unknown", false);
        table_0001.add_value("A", "Ambiguous", false);
        table_0001.add_value("N", "Not applicable", false);
        self.add_table(table_0001);

        // HL7 Table 0002 - Marital Status
        let mut table_0002 = Hl7Table::new("0002", "Marital Status", "Patient's marital status");
        table_0002.add_value("A", "Separated", false);
        table_0002.add_value("D", "Divorced", false);
        table_0002.add_value("M", "Married", false);
        table_0002.add_value("S", "Single", false);
        table_0002.add_value("W", "Widowed", false);
        table_0002.add_value("C", "Common law", false);
        table_0002.add_value("G", "Living together", false);
        table_0002.add_value("P", "Domestic partner", false);
        table_0002.add_value("R", "Registered domestic partner", false);
        table_0002.add_value("E", "Legally separated", false);
        table_0002.add_value("N", "Annulled", false);
        table_0002.add_value("I", "Interlocutory", false);
        table_0002.add_value("B", "Unmarried", false);
        table_0002.add_value("U", "Unknown", false);
        table_0002.add_value("O", "Other", false);
        table_0002.add_value("T", "Unreported", false);
        self.add_table(table_0002);

        // HL7 Table 0004 - Patient Class
        let mut table_0004 = Hl7Table::new("0004", "Patient Class", "Classification of patient encounter");
        table_0004.add_value("E", "Emergency", false);
        table_0004.add_value("I", "Inpatient", false);
        table_0004.add_value("O", "Outpatient", false);
        table_0004.add_value("P", "Preadmit", false);
        table_0004.add_value("R", "Recurring patient", false);
        table_0004.add_value("B", "Obstetrics", false);
        table_0004.add_value("C", "Commercial account", false);
        table_0004.add_value("N", "Not applicable", false);
        table_0004.add_value("U", "Unknown", false);
        self.add_table(table_0004);

        // HL7 Table 0007 - Admission Type
        let mut table_0007 = Hl7Table::new("0007", "Admission Type", "Type of admission");
        table_0007.add_value("A", "Accident", false);
        table_0007.add_value("C", "Elective", false);
        table_0007.add_value("E", "Emergency", false);
        table_0007.add_value("L", "Labor and delivery", false);
        table_0007.add_value("N", "Newborn", false);
        table_0007.add_value("R", "Routine", false);
        table_0007.add_value("U", "Urgent", false);
        self.add_table(table_0007);

        // HL7 Table 0061 - Check Digit Scheme
        let mut table_0061 = Hl7Table::new("0061", "Check Digit Scheme", "Algorithm for check digit");
        table_0061.add_value("M10", "Mod 10 algorithm", false);
        table_0061.add_value("M11", "Mod 11 algorithm", false);
        table_0061.add_value("ISO", "ISO 7064: 1983", false);
        table_0061.add_value("NPI", "Check digit algorithm in the US National Provider Identifier", false);
        self.add_table(table_0061);

        // HL7 Table 0063 - Relationship
        let mut table_0063 = Hl7Table::new("0063", "Relationship", "Patient relationship to insured");
        table_0063.add_value("SEL", "Self", false);
        table_0063.add_value("SPO", "Spouse", false);
        table_0063.add_value("CHD", "Child", false);
        table_0063.add_value("DEP", "Handicapped dependent", false);
        table_0063.add_value("FCH", "Foster child", false);
        table_0063.add_value("GCH", "Grandchild", false);
        table_0063.add_value("NCH", "Natural child", false);
        table_0063.add_value("SCH", "Stepchild", false);
        table_0063.add_value("OTH", "Other", false);
        table_0063.add_value("PAR", "Parent", false);
        table_0063.add_value("EME", "Employee", false);
        table_0063.add_value("EMR", "Emancipated minor", false);
        self.add_table(table_0063);

        // HL7 Table 0078 - Interpretation Codes
        let mut table_0078 = Hl7Table::new("0078", "Interpretation Codes", "Observation interpretation");
        table_0078.add_value("L", "Below low normal", false);
        table_0078.add_value("H", "Above high normal", false);
        table_0078.add_value("LL", "Below lower panic limits", false);
        table_0078.add_value("HH", "Above upper panic limits", false);
        table_0078.add_value("<", "Below absolute low-off instrument scale", false);
        table_0078.add_value(">", "Above absolute high-off instrument scale", false);
        table_0078.add_value("N", "Normal", false);
        table_0078.add_value("A", "Abnormal", false);
        table_0078.add_value("AA", "Very abnormal", false);
        table_0078.add_value("S", "Susceptible", false);
        table_0078.add_value("R", "Resistant", false);
        table_0078.add_value("I", "Intermediate", false);
        table_0078.add_value("MS", "Moderately susceptible", false);
        table_0078.add_value("VS", "Very susceptible", false);
        self.add_table(table_0078);

        // HL7 Table 0085 - Observation Result Status
        let mut table_0085 = Hl7Table::new("0085", "Observation Result Status", "Status of observation result");
        table_0085.add_value("C", "Record coming over is a correction", false);
        table_0085.add_value("D", "Deletes the OBX record", false);
        table_0085.add_value("F", "Final results", false);
        table_0085.add_value("I", "Specimen in lab; results pending", false);
        table_0085.add_value("N", "Not asked", false);
        table_0085.add_value("O", "Order detail description only", false);
        table_0085.add_value("P", "Preliminary results", false);
        table_0085.add_value("R", "Results entered - not verified", false);
        table_0085.add_value("S", "Partial results", false);
        table_0085.add_value("U", "Results status change to final", false);
        table_0085.add_value("W", "Post original as wrong", false);
        table_0085.add_value("X", "Results cannot be obtained", false);
        self.add_table(table_0085);

        // HL7 Table 0103 - Processing ID
        let mut table_0103 = Hl7Table::new("0103", "Processing ID", "Message processing mode");
        table_0103.add_value("D", "Debugging", false);
        table_0103.add_value("P", "Production", false);
        table_0103.add_value("T", "Training", false);
        self.add_table(table_0103);

        // HL7 Table 0119 - Order Control Codes
        let mut table_0119 = Hl7Table::new("0119", "Order Control Codes", "Order control actions");
        table_0119.add_value("NW", "New order", false);
        table_0119.add_value("OK", "Order accepted & OK", false);
        table_0119.add_value("UA", "Unable to accept order", false);
        table_0119.add_value("CA", "Cancel order request", false);
        table_0119.add_value("OC", "Order canceled", false);
        table_0119.add_value("CR", "Canceled as requested", false);
        table_0119.add_value("UC", "Unable to cancel", false);
        table_0119.add_value("DC", "Discontinue order request", false);
        table_0119.add_value("OD", "Order discontinued", false);
        table_0119.add_value("DR", "Discontinued as requested", false);
        table_0119.add_value("UD", "Unable to discontinue", false);
        table_0119.add_value("HD", "Hold order request", false);
        table_0119.add_value("OH", "Order held", false);
        table_0119.add_value("UH", "Unable to put on hold", false);
        table_0119.add_value("RL", "Release previous hold", false);
        table_0119.add_value("OE", "Order released", false);
        table_0119.add_value("OR", "Released as requested", false);
        table_0119.add_value("UR", "Unable to release", false);
        table_0119.add_value("RP", "Order replace request", false);
        table_0119.add_value("RU", "Replaced unsolicited", false);
        table_0119.add_value("RO", "Replacement order", false);
        table_0119.add_value("RQ", "Replaced as requested", false);
        table_0119.add_value("UM", "Unable to replace", false);
        table_0119.add_value("PA", "Parent order", false);
        table_0119.add_value("CH", "Child order", false);
        table_0119.add_value("XO", "Change order request", false);
        table_0119.add_value("XX", "Order changed", false);
        table_0119.add_value("UX", "Unable to change", false);
        table_0119.add_value("DE", "Data errors", false);
        table_0119.add_value("RE", "Observations to follow", false);
        table_0119.add_value("RR", "Request received", false);
        table_0119.add_value("SR", "Response to send order status request", false);
        table_0119.add_value("SS", "Send order status request", false);
        table_0119.add_value("SC", "Status changed", false);
        table_0119.add_value("SN", "Send order number", false);
        table_0119.add_value("NA", "Number assigned", false);
        table_0119.add_value("CN", "Combined result", false);
        table_0119.add_value("XR", "Changed as requested", false);
        self.add_table(table_0119);

        // HL7 Table 0201 - Telecommunication Use Code
        let mut table_0201 = Hl7Table::new("0201", "Telecommunication Use Code", "Purpose of telecom");
        table_0201.add_value("PRN", "Primary residence number", false);
        table_0201.add_value("ORN", "Other residence number", false);
        table_0201.add_value("WPN", "Work number", false);
        table_0201.add_value("VHN", "Vacation home number", false);
        table_0201.add_value("ASN", "Answering service number", false);
        table_0201.add_value("EMR", "Emergency number", false);
        table_0201.add_value("NET", "Network (email) address", false);
        table_0201.add_value("BPN", "Beeper number", false);
        self.add_table(table_0201);

        // HL7 Table 0203 - Identifier Type
        let mut table_0203 = Hl7Table::new("0203", "Identifier Type", "Type of identifier");
        table_0203.add_value("AM", "American Express", false);
        table_0203.add_value("AN", "Account number", false);
        table_0203.add_value("BR", "Birth registry number", false);
        table_0203.add_value("DI", "Diner's Club card", false);
        table_0203.add_value("DL", "Driver's license number", false);
        table_0203.add_value("DN", "Doctor number", false);
        table_0203.add_value("DS", "Discover Card", false);
        table_0203.add_value("EI", "Employee number", false);
        table_0203.add_value("EN", "Employer number", false);
        table_0203.add_value("MA", "Medicaid number", false);
        table_0203.add_value("MC", "Medicare number", false);
        table_0203.add_value("MR", "Medical record number", false);
        table_0203.add_value("MS", "MasterCard", false);
        table_0203.add_value("NE", "National employer identifier", false);
        table_0203.add_value("NH", "National Health Plan Identifier", false);
        table_0203.add_value("NI", "National unique individual identifier", false);
        table_0203.add_value("NNxxx", "National Person Identifier", false);
        table_0203.add_value("NPI", "National provider identifier", false);
        table_0203.add_value("PEN", "Pension number", false);
        table_0203.add_value("PI", "Patient internal identifier", false);
        table_0203.add_value("PN", "Person number", false);
        table_0203.add_value("RR", "Railroad retirement number", false);
        table_0203.add_value("SN", "Subscriber number", false);
        table_0203.add_value("SR", "State registry ID", false);
        table_0203.add_value("SS", "Social Security number", false);
        table_0203.add_value("U", "Unspecified identifier", false);
        table_0203.add_value("VS", "VISA", false);
        table_0203.add_value("VN", "Visit number", false);
        table_0203.add_value("XX", "Organization identifier", false);
        self.add_table(table_0203);

        // HL7 Table 0301 - Universal ID Type
        let mut table_0301 = Hl7Table::new("0301", "Universal ID Type", "Type of universal identifier");
        table_0301.add_value("DNS", "Domain Name System", false);
        table_0301.add_value("GUID", "Globally Unique Identifier", false);
        table_0301.add_value("HCD", "CEN Healthcare Coding Identifier", false);
        table_0301.add_value("HL7", "HL7 registration schemes", false);
        table_0301.add_value("ISO", "ISO Object Identifier", false);
        table_0301.add_value("L", "Local", false);
        table_0301.add_value("M", "MAC address", false);
        table_0301.add_value("N", "Node", false);
        table_0301.add_value("Random", "Random", false);
        table_0301.add_value("URI", "Uniform Resource Identifier", false);
        table_0301.add_value("UUID", "Universal Unique Identifier", false);
        table_0301.add_value("x400", "X.400 MHS identifier", false);
        table_0301.add_value("x500", "X.500 directory Name", false);
        self.add_table(table_0301);
    }
}

impl Default for TableRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_administrative_sex() {
        let registry = TableRegistry::new();
        assert!(registry.validate("0001", "M").is_valid());
        assert!(registry.validate("0001", "F").is_valid());
        assert!(registry.validate("0001", "U").is_valid());
        assert!(!registry.validate("0001", "X").is_valid());
    }

    #[test]
    fn test_patient_class() {
        let registry = TableRegistry::new();
        assert!(registry.validate("0004", "I").is_valid());
        assert!(registry.validate("0004", "O").is_valid());
        assert!(registry.validate("0004", "E").is_valid());
        assert!(!registry.validate("0004", "Z").is_valid());
    }

    #[test]
    fn test_processing_id() {
        let registry = TableRegistry::new();
        assert!(registry.validate("0103", "P").is_valid());
        assert!(registry.validate("0103", "D").is_valid());
        assert!(registry.validate("0103", "T").is_valid());
        assert!(!registry.validate("0103", "X").is_valid());
    }

    #[test]
    fn test_observation_result_status() {
        let registry = TableRegistry::new();
        assert!(registry.validate("0085", "F").is_valid());
        assert!(registry.validate("0085", "P").is_valid());
        assert!(registry.validate("0085", "C").is_valid());
        assert!(!registry.validate("0085", "Z").is_valid());
    }

    #[test]
    fn test_empty_value() {
        let registry = TableRegistry::new();
        assert!(registry.validate("0001", "").is_valid());
    }

    #[test]
    fn test_unknown_table() {
        let registry = TableRegistry::new();
        let result = registry.validate("9999", "TEST");
        assert!(result.is_valid()); // NotApplicable is considered valid
    }

    #[test]
    fn test_order_control_codes() {
        let registry = TableRegistry::new();
        assert!(registry.validate("0119", "NW").is_valid());
        assert!(registry.validate("0119", "CA").is_valid());
        assert!(registry.validate("0119", "OK").is_valid());
        assert!(!registry.validate("0119", "INVALID").is_valid());
    }

    #[test]
    fn test_custom_table() {
        let mut registry = TableRegistry::new();

        let mut custom_table = Hl7Table::new("9000", "Custom Table", "Test table");
        custom_table.add_value("A", "Option A", false);
        custom_table.add_value("B", "Option B", false);

        registry.add_table(custom_table);

        assert!(registry.validate("9000", "A").is_valid());
        assert!(registry.validate("9000", "B").is_valid());
        assert!(!registry.validate("9000", "C").is_valid());
    }
}
