//! Built-in common validation rules
//!
//! This module provides pre-built validation rules for common HL7 validation scenarios.
//! Rules are organized by message type and segment.
//!
//! ## Usage
//!
//! ```
//! use vyna_hl7_validator::rules::builtin::BuiltinRules;
//! use vyna_hl7_validator::rules::RulesEngine;
//!
//! let mut engine = RulesEngine::new();
//!
//! // Add all PID segment rules
//! engine.add_rules(BuiltinRules::pid_rules());
//!
//! // Add all MSH segment rules
//! engine.add_rules(BuiltinRules::msh_rules());
//!
//! // Or add all ADT message rules
//! engine.add_rules(BuiltinRules::adt_rules());
//! ```

use super::{CrossFieldValidator, RuleSeverity, ValidationRule};

/// Built-in validation rules for common HL7 scenarios
pub struct BuiltinRules;

impl BuiltinRules {
    /// Get all MSH (Message Header) segment rules
    ///
    /// Includes:
    /// - MSH-3 (Sending Application) must be valued
    /// - MSH-4 (Sending Facility) must be valued
    /// - MSH-9 (Message Type) must be valued
    /// - MSH-10 (Message Control ID) must be valued
    /// - MSH-11 (Processing ID) must be valued and in valid set
    /// - MSH-12 (Version ID) must be valued
    pub fn msh_rules() -> Vec<ValidationRule> {
        vec![
            CrossFieldValidator::field_valued(
                "msh_sending_application_required",
                "Sending Application (MSH-3) must be provided",
                RuleSeverity::Error,
                "MSH-3",
            ),
            CrossFieldValidator::field_valued(
                "msh_sending_facility_required",
                "Sending Facility (MSH-4) must be provided",
                RuleSeverity::Error,
                "MSH-4",
            ),
            CrossFieldValidator::field_valued(
                "msh_message_type_required",
                "Message Type (MSH-9) must be provided",
                RuleSeverity::Error,
                "MSH-9",
            ),
            CrossFieldValidator::field_valued(
                "msh_message_control_id_required",
                "Message Control ID (MSH-10) must be provided",
                RuleSeverity::Error,
                "MSH-10",
            ),
            CrossFieldValidator::field_in_set(
                "msh_processing_id_valid",
                "Processing ID (MSH-11) must be P (Production), T (Test), or D (Debug)",
                RuleSeverity::Error,
                "MSH-11",
                vec!["P", "T", "D"],
            ),
            CrossFieldValidator::field_valued(
                "msh_version_id_required",
                "Version ID (MSH-12) must be provided",
                RuleSeverity::Warning,
                "MSH-12",
            ),
        ]
    }

    /// Get all PID (Patient Identification) segment rules
    ///
    /// Includes:
    /// - At least one patient ID (PID-2, PID-3, or PID-4) must be valued
    /// - PID-5 (Patient Name) must be valued
    /// - PID-7 (Date of Birth) should be valued
    /// - PID-8 (Administrative Sex) should be valued and in valid set
    pub fn pid_rules() -> Vec<ValidationRule> {
        vec![
            CrossFieldValidator::at_least_one(
                "pid_patient_id_required",
                "At least one patient identifier (PID-2, PID-3, or PID-4) must be provided",
                RuleSeverity::Error,
                vec!["PID-2", "PID-3", "PID-4"],
            ),
            CrossFieldValidator::field_valued(
                "pid_patient_name_required",
                "Patient Name (PID-5) must be provided",
                RuleSeverity::Error,
                "PID-5",
            ),
            CrossFieldValidator::field_valued(
                "pid_date_of_birth_required",
                "Patient Date of Birth (PID-7) should be provided",
                RuleSeverity::Warning,
                "PID-7",
            ),
            CrossFieldValidator::field_in_set(
                "pid_administrative_sex_valid",
                "Administrative Sex (PID-8) must be F, M, O, U, A, or N",
                RuleSeverity::Warning,
                "PID-8",
                vec!["F", "M", "O", "U", "A", "N"],
            ),
        ]
    }

    /// Get all PV1 (Patient Visit) segment rules
    ///
    /// Includes:
    /// - PV1-2 (Patient Class) must be valued and in valid set
    /// - PV1-3 (Assigned Patient Location) should be valued for inpatient
    /// - PV1-7 (Attending Doctor) should be valued
    /// - PV1-19 (Visit Number) should be valued
    pub fn pv1_rules() -> Vec<ValidationRule> {
        vec![
            CrossFieldValidator::field_in_set(
                "pv1_patient_class_valid",
                "Patient Class (PV1-2) must be E (Emergency), I (Inpatient), O (Outpatient), P (Preadmit), R (Recurring), or B (Obstetrics)",
                RuleSeverity::Error,
                "PV1-2",
                vec!["E", "I", "O", "P", "R", "B", "C", "N", "U"],
            ),
            CrossFieldValidator::if_then(
                "pv1_inpatient_needs_location",
                "Inpatient (PV1-2 = I) must have Assigned Patient Location (PV1-3)",
                RuleSeverity::Warning,
                "PV1-2",
                "I",
                "PV1-3",
            ),
            CrossFieldValidator::field_valued(
                "pv1_attending_doctor_required",
                "Attending Doctor (PV1-7) should be provided",
                RuleSeverity::Warning,
                "PV1-7",
            ),
            CrossFieldValidator::field_valued(
                "pv1_visit_number_required",
                "Visit Number (PV1-19) should be provided",
                RuleSeverity::Warning,
                "PV1-19",
            ),
        ]
    }

    /// Get all OBR (Observation Request) segment rules
    ///
    /// Includes:
    /// - OBR-4 (Universal Service ID) must be valued
    /// - OBR-7 (Observation Date/Time) must be valued
    /// - OBR-16 (Ordering Provider) should be valued
    /// - OBR-25 (Result Status) must be in valid set
    pub fn obr_rules() -> Vec<ValidationRule> {
        vec![
            CrossFieldValidator::field_valued(
                "obr_universal_service_id_required",
                "Universal Service ID (OBR-4) must be provided",
                RuleSeverity::Error,
                "OBR-4",
            ),
            CrossFieldValidator::field_valued(
                "obr_observation_datetime_required",
                "Observation Date/Time (OBR-7) must be provided",
                RuleSeverity::Error,
                "OBR-7",
            ),
            CrossFieldValidator::field_valued(
                "obr_ordering_provider_required",
                "Ordering Provider (OBR-16) should be provided",
                RuleSeverity::Warning,
                "OBR-16",
            ),
            CrossFieldValidator::field_in_set(
                "obr_result_status_valid",
                "Result Status (OBR-25) must be A, C, F, I, O, P, R, S, X, or Y",
                RuleSeverity::Warning,
                "OBR-25",
                vec!["A", "C", "F", "I", "O", "P", "R", "S", "X", "Y"],
            ),
        ]
    }

    /// Get all OBX (Observation/Result) segment rules
    ///
    /// Includes:
    /// - OBX-2 (Value Type) must be valued and in valid set
    /// - OBX-3 (Observation Identifier) must be valued
    /// - OBX-5 (Observation Value) must be valued
    /// - OBX-11 (Observation Result Status) must be valued and in valid set
    pub fn obx_rules() -> Vec<ValidationRule> {
        vec![
            CrossFieldValidator::field_in_set(
                "obx_value_type_valid",
                "Value Type (OBX-2) must be CE, CF, CK, CN, CP, CX, DT, ED, FT, MO, NM, PN, RP, SN, ST, TM, TN, TS, TX, or XAD",
                RuleSeverity::Error,
                "OBX-2",
                vec![
                    "CE", "CF", "CK", "CN", "CP", "CX", "DT", "ED", "FT", "MO", "NM", "PN", "RP",
                    "SN", "ST", "TM", "TN", "TS", "TX", "XAD", "XCN", "XON", "XPN", "XTN",
                ],
            ),
            CrossFieldValidator::field_valued(
                "obx_observation_identifier_required",
                "Observation Identifier (OBX-3) must be provided",
                RuleSeverity::Error,
                "OBX-3",
            ),
            CrossFieldValidator::field_valued(
                "obx_observation_value_required",
                "Observation Value (OBX-5) must be provided",
                RuleSeverity::Error,
                "OBX-5",
            ),
            CrossFieldValidator::field_in_set(
                "obx_result_status_valid",
                "Observation Result Status (OBX-11) must be C, D, F, I, N, O, P, R, S, U, W, or X",
                RuleSeverity::Error,
                "OBX-11",
                vec!["C", "D", "F", "I", "N", "O", "P", "R", "S", "U", "W", "X"],
            ),
        ]
    }

    /// Get all ORC (Common Order) segment rules
    ///
    /// Includes:
    /// - ORC-1 (Order Control) must be valued and in valid set
    /// - ORC-2 (Placer Order Number) or ORC-3 (Filler Order Number) must be valued
    pub fn orc_rules() -> Vec<ValidationRule> {
        vec![
            CrossFieldValidator::field_in_set(
                "orc_order_control_valid",
                "Order Control (ORC-1) must be a valid code (NW, OK, UA, CA, OC, CR, DC, DE, DF, DR, FU, HD, HR, LI, NA, NW, OC, OD, OE, OF, OH, OK, OP, OR, PA, PR, RE, RF, RL, RO, RP, RQ, RR, RU, SC, SN, SR, SS, UA, UC, UD, UF, UH, UM, UN, UP, UR, UX, XO, XR, XX)",
                RuleSeverity::Error,
                "ORC-1",
                vec![
                    "NW", "OK", "UA", "CA", "OC", "CR", "DC", "DE", "DF", "DR", "FU", "HD", "HR",
                    "LI", "NA", "OD", "OE", "OF", "OH", "OP", "OR", "PA", "PR", "RE", "RF", "RL",
                    "RO", "RP", "RQ", "RR", "RU", "SC", "SN", "SR", "SS", "UC", "UD", "UF", "UH",
                    "UM", "UN", "UP", "UR", "UX", "XO", "XR", "XX",
                ],
            ),
            CrossFieldValidator::at_least_one(
                "orc_order_number_required",
                "At least one of Placer Order Number (ORC-2) or Filler Order Number (ORC-3) must be provided",
                RuleSeverity::Error,
                vec!["ORC-2", "ORC-3"],
            ),
        ]
    }

    /// Get all rules for ADT (Admission/Discharge/Transfer) messages
    ///
    /// Includes all MSH, PID, and PV1 rules
    pub fn adt_rules() -> Vec<ValidationRule> {
        let mut rules = Vec::new();
        rules.extend(Self::msh_rules());
        rules.extend(Self::pid_rules());
        rules.extend(Self::pv1_rules());
        rules
    }

    /// Get all rules for ORU (Observation Result) messages
    ///
    /// Includes all MSH, PID, OBR, and OBX rules
    pub fn oru_rules() -> Vec<ValidationRule> {
        let mut rules = Vec::new();
        rules.extend(Self::msh_rules());
        rules.extend(Self::pid_rules());
        rules.extend(Self::obr_rules());
        rules.extend(Self::obx_rules());
        rules
    }

    /// Get all rules for ORM (Order) messages
    ///
    /// Includes all MSH, PID, and ORC rules
    pub fn orm_rules() -> Vec<ValidationRule> {
        let mut rules = Vec::new();
        rules.extend(Self::msh_rules());
        rules.extend(Self::pid_rules());
        rules.extend(Self::orc_rules());
        rules
    }

    /// Get all available built-in rules
    ///
    /// This includes all segment-specific rules without duplication
    pub fn all_rules() -> Vec<ValidationRule> {
        let mut rules = Vec::new();
        rules.extend(Self::msh_rules());
        rules.extend(Self::pid_rules());
        rules.extend(Self::pv1_rules());
        rules.extend(Self::obr_rules());
        rules.extend(Self::obx_rules());
        rules.extend(Self::orc_rules());
        rules
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_msh_rules_count() {
        let rules = BuiltinRules::msh_rules();
        assert_eq!(rules.len(), 6);
    }

    #[test]
    fn test_pid_rules_count() {
        let rules = BuiltinRules::pid_rules();
        assert_eq!(rules.len(), 4);
    }

    #[test]
    fn test_pv1_rules_count() {
        let rules = BuiltinRules::pv1_rules();
        assert_eq!(rules.len(), 4);
    }

    #[test]
    fn test_obr_rules_count() {
        let rules = BuiltinRules::obr_rules();
        assert_eq!(rules.len(), 4);
    }

    #[test]
    fn test_obx_rules_count() {
        let rules = BuiltinRules::obx_rules();
        assert_eq!(rules.len(), 4);
    }

    #[test]
    fn test_orc_rules_count() {
        let rules = BuiltinRules::orc_rules();
        assert_eq!(rules.len(), 2);
    }

    #[test]
    fn test_adt_rules_count() {
        let rules = BuiltinRules::adt_rules();
        // MSH (6) + PID (4) + PV1 (4) = 14
        assert_eq!(rules.len(), 14);
    }

    #[test]
    fn test_oru_rules_count() {
        let rules = BuiltinRules::oru_rules();
        // MSH (6) + PID (4) + OBR (4) + OBX (4) = 18
        assert_eq!(rules.len(), 18);
    }

    #[test]
    fn test_orm_rules_count() {
        let rules = BuiltinRules::orm_rules();
        // MSH (6) + PID (4) + ORC (2) = 12
        assert_eq!(rules.len(), 12);
    }

    #[test]
    fn test_all_rules_count() {
        let rules = BuiltinRules::all_rules();
        // MSH (6) + PID (4) + PV1 (4) + OBR (4) + OBX (4) + ORC (2) = 24
        assert_eq!(rules.len(), 24);
    }

    #[test]
    fn test_rule_names_are_unique() {
        let rules = BuiltinRules::all_rules();
        let mut names = std::collections::HashSet::new();

        for rule in &rules {
            assert!(
                names.insert(rule.name.clone()),
                "Duplicate rule name: {}",
                rule.name
            );
        }
    }

    #[test]
    fn test_msh_rules_have_correct_severity() {
        let rules = BuiltinRules::msh_rules();

        // Most MSH rules should be errors
        let error_count = rules
            .iter()
            .filter(|r| r.severity == RuleSeverity::Error)
            .count();
        assert!(error_count >= 4);
    }

    #[test]
    fn test_pid_rules_have_mixed_severity() {
        let rules = BuiltinRules::pid_rules();

        // Should have both errors and warnings
        let has_errors = rules.iter().any(|r| r.severity == RuleSeverity::Error);
        let has_warnings = rules.iter().any(|r| r.severity == RuleSeverity::Warning);

        assert!(has_errors);
        assert!(has_warnings);
    }
}
