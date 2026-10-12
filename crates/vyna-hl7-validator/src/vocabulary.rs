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
        matches!(
            self,
            VocabularyValidation::Valid | VocabularyValidation::NotApplicable
        )
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
    /// Open-ended code patterns in HL7 spelling (e.g. `NNxxx`, `HL7nnnn`,
    /// `Q<integer>D`, `>=2`). See [`Hl7Table::add_pattern`].
    #[serde(default)]
    pub patterns: Vec<String>,
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
            patterns: Vec::new(),
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

    /// Add an open-ended code pattern in HL7 spelling.
    ///
    /// Supported pattern elements:
    /// - `>=N`: a purely numeric code with value `>= N` (e.g. `>=2`).
    /// - `<integer>`, `<day#>`: one or more digits (e.g. `Q<integer>D`).
    /// - `x`/`z` runs: exactly that many ASCII alphanumerics (`NNxxx`).
    /// - `n` runs: exactly that many ASCII digits (`HL7nnnn`).
    /// - `s` runs: exactly that many ASCII alphanumerics (`NCPDPnnnnsss`).
    /// - Anything else matches literally (case-sensitive).
    ///
    /// Uppercase `X`/`Z` only act as classes in runs of two or more, so a
    /// literal `X` (as in `X12Dennnn`) keeps working.
    pub fn add_pattern(&mut self, pattern: &str) {
        self.patterns.push(pattern.to_string());
    }

    /// Validate a code against this table
    pub fn validate(&self, code: &str) -> VocabularyValidation {
        if code.is_empty() {
            return VocabularyValidation::Valid;
        }

        if !self.values.contains_key(code) && self.patterns.iter().any(|p| pattern_matches(p, code))
        {
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
        codes.extend(self.patterns.iter().cloned());
        codes.sort();
        if codes.len() > 10 {
            format!("{}, ... ({} total)", codes[..10].join(", "), codes.len())
        } else {
            codes.join(", ")
        }
    }
}

/// One element of a compiled [`Hl7Table::add_pattern`] pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PatternToken {
    /// Literal character (case-sensitive).
    Literal(char),
    /// One or more ASCII digits (`<integer>`, `<day#>`).
    Digits,
    /// Exactly `n` ASCII digits (`n` run).
    DigitRun(usize),
    /// Exactly `n` ASCII alphanumerics (`x`/`z`/`s` run).
    AlnumRun(usize),
}

/// Match a code against an HL7 pattern (see [`Hl7Table::add_pattern`]).
fn pattern_matches(pattern: &str, code: &str) -> bool {
    if let Some(min) = pattern.strip_prefix(">=") {
        let Ok(min) = min.parse::<u64>() else {
            return false;
        };
        return !code.is_empty()
            && code.bytes().all(|b| b.is_ascii_digit())
            && code.parse::<u64>().is_ok_and(|v| v >= min);
    }
    let tokens = tokenize_pattern(pattern);
    match_tokens(&tokens, code.as_bytes())
}

fn tokenize_pattern(pattern: &str) -> Vec<PatternToken> {
    let mut tokens = Vec::new();
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            let group: String = chars.by_ref().take_while(|&g| g != '>').collect();
            let group = group.replace(' ', "");
            if group.eq_ignore_ascii_case("integer") || group.eq_ignore_ascii_case("day#") {
                tokens.push(PatternToken::Digits);
                continue;
            }
            // Unknown group: match literally.
            tokens.push(PatternToken::Literal('<'));
            tokens.extend(group.chars().map(PatternToken::Literal));
            tokens.push(PatternToken::Literal('>'));
            continue;
        }
        let class = class_of(c);
        if let Some(kind) = class {
            let mut run = 1;
            while chars.peek().is_some_and(|&n| class_of(n) == Some(kind)) {
                chars.next();
                run += 1;
            }
            // Single uppercase X/Z stays literal (e.g. the X in X12Dennnn).
            if run == 1 && (c == 'X' || c == 'Z') {
                tokens.push(PatternToken::Literal(c));
            } else {
                tokens.push(match kind {
                    CharClass::Digit => PatternToken::DigitRun(run),
                    CharClass::Alnum => PatternToken::AlnumRun(run),
                });
            }
            continue;
        }
        tokens.push(PatternToken::Literal(c));
    }
    tokens
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharClass {
    Digit,
    Alnum,
}

/// Pattern class of a character, if it is one.
fn class_of(c: char) -> Option<CharClass> {
    match c {
        'n' => Some(CharClass::Digit),
        'x' | 'z' | 's' | 'X' | 'Z' => Some(CharClass::Alnum),
        _ => None,
    }
}

/// Match pre-tokenized pattern against code bytes (full match).
fn match_tokens(tokens: &[PatternToken], code: &[u8]) -> bool {
    fn go(tokens: &[PatternToken], code: &[u8]) -> bool {
        let Some((first, rest)) = tokens.split_first() else {
            return code.is_empty();
        };
        match *first {
            PatternToken::Literal(c) => {
                let mut buf = [0u8; 4];
                let encoded = c.encode_utf8(&mut buf).as_bytes();
                code.strip_prefix(encoded)
                    .is_some_and(|tail| go(rest, tail))
            }
            PatternToken::DigitRun(n) | PatternToken::AlnumRun(n) => {
                if code.len() < n {
                    return false;
                }
                let (head, tail) = code.split_at(n);
                let ok = head.iter().all(|b| {
                    if matches!(*first, PatternToken::DigitRun(_)) {
                        b.is_ascii_digit()
                    } else {
                        b.is_ascii_alphanumeric()
                    }
                });
                ok && go(rest, tail)
            }
            PatternToken::Digits => {
                // One or more digits, with backtracking.
                let count = code.iter().take_while(|b| b.is_ascii_digit()).count();
                (1..=count).rev().any(|n| go(rest, &code[n..]))
            }
        }
    }
    go(tokens, code)
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
    ///
    /// Values are the union of hl7apy table data across v2.3.1-v2.8.2,
    /// curated (range expansion, pattern support, user-table exclusions).
    /// Tables without HL7-defined values stay unloaded so fields that
    /// reference them validate as NotApplicable.
    fn load_standard_tables(&mut self) {
        // HL7 Table 0001 - Administrative Sex
        let mut table_0001 =
            Hl7Table::new("0001", "Administrative Sex", "Patient's administrative sex");
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

        // HL7 Table 0003 - Event Type
        let mut table_0003 = Hl7Table::new("0003", "Event Type", "Event Type");
        table_0003.add_value("A01", "A01", false);
        table_0003.add_value("A02", "A02", false);
        table_0003.add_value("A03", "A03", false);
        table_0003.add_value("A04", "A04", false);
        table_0003.add_value("A05", "A05", false);
        table_0003.add_value("A06", "A06", false);
        table_0003.add_value("A07", "A07", false);
        table_0003.add_value("A08", "A08", false);
        table_0003.add_value("A09", "A09", false);
        table_0003.add_value("A10", "A10", false);
        table_0003.add_value("A11", "A11", false);
        table_0003.add_value("A12", "A12", false);
        table_0003.add_value("A13", "A13", false);
        table_0003.add_value("A14", "A14", false);
        table_0003.add_value("A15", "A15", false);
        table_0003.add_value("A16", "A16", false);
        table_0003.add_value("A17", "A17", false);
        table_0003.add_value("A18", "A18", false);
        table_0003.add_value("A19", "A19", false);
        table_0003.add_value("A20", "A20", false);
        table_0003.add_value("A21", "A21", false);
        table_0003.add_value("A22", "A22", false);
        table_0003.add_value("A23", "A23", false);
        table_0003.add_value("A24", "A24", false);
        table_0003.add_value("A25", "A25", false);
        table_0003.add_value("A26", "A26", false);
        table_0003.add_value("A27", "A27", false);
        table_0003.add_value("A28", "A28", false);
        table_0003.add_value("A29", "A29", false);
        table_0003.add_value("A30", "A30", false);
        table_0003.add_value("A31", "A31", false);
        table_0003.add_value("A32", "A32", false);
        table_0003.add_value("A33", "A33", false);
        table_0003.add_value("A34", "A34", false);
        table_0003.add_value("A35", "A35", false);
        table_0003.add_value("A36", "A36", false);
        table_0003.add_value("A37", "A37", false);
        table_0003.add_value("A38", "A38", false);
        table_0003.add_value("A39", "A39", false);
        table_0003.add_value("A40", "A40", false);
        table_0003.add_value("A41", "A41", false);
        table_0003.add_value("A42", "A42", false);
        table_0003.add_value("A43", "A43", false);
        table_0003.add_value("A44", "A44", false);
        table_0003.add_value("A45", "A45", false);
        table_0003.add_value("A46", "A46", false);
        table_0003.add_value("A47", "A47", false);
        table_0003.add_value("A48", "A48", false);
        table_0003.add_value("A49", "A49", false);
        table_0003.add_value("A50", "A50", false);
        table_0003.add_value("A51", "A51", false);
        table_0003.add_value("A52", "A52", false);
        table_0003.add_value("A53", "A53", false);
        table_0003.add_value("A54", "A54", false);
        table_0003.add_value("A55", "A55", false);
        table_0003.add_value("A60", "A60", false);
        table_0003.add_value("A61", "A61", false);
        table_0003.add_value("A62", "A62", false);
        table_0003.add_value("B01", "B01", false);
        table_0003.add_value("B02", "B02", false);
        table_0003.add_value("B03", "B03", false);
        table_0003.add_value("B04", "B04", false);
        table_0003.add_value("B05", "B05", false);
        table_0003.add_value("B06", "B06", false);
        table_0003.add_value("B07", "B07", false);
        table_0003.add_value("B08", "B08", false);
        table_0003.add_value("C01", "C01", false);
        table_0003.add_value("C02", "C02", false);
        table_0003.add_value("C03", "C03", false);
        table_0003.add_value("C04", "C04", false);
        table_0003.add_value("C05", "C05", false);
        table_0003.add_value("C06", "C06", false);
        table_0003.add_value("C07", "C07", false);
        table_0003.add_value("C08", "C08", false);
        table_0003.add_value("C09", "C09", false);
        table_0003.add_value("C10", "C10", false);
        table_0003.add_value("C11", "C11", false);
        table_0003.add_value("C12", "C12", false);
        table_0003.add_value("CNQ", "CNQ", false);
        table_0003.add_value("E01", "E01", false);
        table_0003.add_value("E02", "E02", false);
        table_0003.add_value("E03", "E03", false);
        table_0003.add_value("E04", "E04", false);
        table_0003.add_value("E10", "E10", false);
        table_0003.add_value("E12", "E12", false);
        table_0003.add_value("E13", "E13", false);
        table_0003.add_value("E15", "E15", false);
        table_0003.add_value("E20", "E20", false);
        table_0003.add_value("E21", "E21", false);
        table_0003.add_value("E22", "E22", false);
        table_0003.add_value("E24", "E24", false);
        table_0003.add_value("E30", "E30", false);
        table_0003.add_value("E31", "E31", false);
        table_0003.add_value("I01", "I01", false);
        table_0003.add_value("I02", "I02", false);
        table_0003.add_value("I03", "I03", false);
        table_0003.add_value("I04", "I04", false);
        table_0003.add_value("I05", "I05", false);
        table_0003.add_value("I06", "I06", false);
        table_0003.add_value("I07", "I07", false);
        table_0003.add_value("I08", "I08", false);
        table_0003.add_value("I09", "I09", false);
        table_0003.add_value("I10", "I10", false);
        table_0003.add_value("I11", "I11", false);
        table_0003.add_value("I12", "I12", false);
        table_0003.add_value("I13", "I13", false);
        table_0003.add_value("I14", "I14", false);
        table_0003.add_value("I15", "I15", false);
        table_0003.add_value("I16", "I16", false);
        table_0003.add_value("I17", "I17", false);
        table_0003.add_value("I18", "I18", false);
        table_0003.add_value("I19", "I19", false);
        table_0003.add_value("I20", "I20", false);
        table_0003.add_value("I21", "I21", false);
        table_0003.add_value("I22", "I22", false);
        table_0003.add_value("J01", "J01", false);
        table_0003.add_value("J02", "J02", false);
        table_0003.add_value("K11", "K11", false);
        table_0003.add_value("K13", "K13", false);
        table_0003.add_value("K15", "K15", false);
        table_0003.add_value("K21", "K21", false);
        table_0003.add_value("K22", "K22", false);
        table_0003.add_value("K23", "K23", false);
        table_0003.add_value("K24", "K24", false);
        table_0003.add_value("K25", "K25", false);
        table_0003.add_value("K31", "K31", false);
        table_0003.add_value("K32", "K32", false);
        table_0003.add_value("K33", "K33", false);
        table_0003.add_value("K34", "K34", false);
        table_0003.add_value("M01", "M01", false);
        table_0003.add_value("M02", "M02", false);
        table_0003.add_value("M03", "M03", false);
        table_0003.add_value("M04", "M04", false);
        table_0003.add_value("M05", "M05", false);
        table_0003.add_value("M06", "M06", false);
        table_0003.add_value("M07", "M07", false);
        table_0003.add_value("M08", "M08", false);
        table_0003.add_value("M09", "M09", false);
        table_0003.add_value("M10", "M10", false);
        table_0003.add_value("M11", "M11", false);
        table_0003.add_value("M12", "M12", false);
        table_0003.add_value("M13", "M13", false);
        table_0003.add_value("M14", "M14", false);
        table_0003.add_value("M15", "M15", false);
        table_0003.add_value("M16", "M16", false);
        table_0003.add_value("M17", "M17", false);
        table_0003.add_value("M18", "M18", false);
        table_0003.add_value("N01", "N01", false);
        table_0003.add_value("N02", "N02", false);
        table_0003.add_value("O01", "O01", false);
        table_0003.add_value("O02", "O02", false);
        table_0003.add_value("O03", "O03", false);
        table_0003.add_value("O04", "O04", false);
        table_0003.add_value("O05", "O05", false);
        table_0003.add_value("O06", "O06", false);
        table_0003.add_value("O07", "O07", false);
        table_0003.add_value("O08", "O08", false);
        table_0003.add_value("O09", "O09", false);
        table_0003.add_value("O10", "O10", false);
        table_0003.add_value("O11", "O11", false);
        table_0003.add_value("O12", "O12", false);
        table_0003.add_value("O13", "O13", false);
        table_0003.add_value("O14", "O14", false);
        table_0003.add_value("O15", "O15", false);
        table_0003.add_value("O16", "O16", false);
        table_0003.add_value("O17", "O17", false);
        table_0003.add_value("O18", "O18", false);
        table_0003.add_value("O19", "O19", false);
        table_0003.add_value("O20", "O20", false);
        table_0003.add_value("O21", "O21", false);
        table_0003.add_value("O22", "O22", false);
        table_0003.add_value("O23", "O23", false);
        table_0003.add_value("O24", "O24", false);
        table_0003.add_value("O25", "O25", false);
        table_0003.add_value("O26", "O26", false);
        table_0003.add_value("O27", "O27", false);
        table_0003.add_value("O28", "O28", false);
        table_0003.add_value("O29", "O29", false);
        table_0003.add_value("O30", "O30", false);
        table_0003.add_value("O31", "O31", false);
        table_0003.add_value("O32", "O32", false);
        table_0003.add_value("O33", "O33", false);
        table_0003.add_value("O34", "O34", false);
        table_0003.add_value("O35", "O35", false);
        table_0003.add_value("O36", "O36", false);
        table_0003.add_value("O37", "O37", false);
        table_0003.add_value("O38", "O38", false);
        table_0003.add_value("O39", "O39", false);
        table_0003.add_value("O40", "O40", false);
        table_0003.add_value("O41", "O41", false);
        table_0003.add_value("O42", "O42", false);
        table_0003.add_value("O43", "O43", false);
        table_0003.add_value("O44", "O44", false);
        table_0003.add_value("O45", "O45", false);
        table_0003.add_value("O46", "O46", false);
        table_0003.add_value("O47", "O47", false);
        table_0003.add_value("O48", "O48", false);
        table_0003.add_value("P01", "P01", false);
        table_0003.add_value("P02", "P02", false);
        table_0003.add_value("P03", "P03", false);
        table_0003.add_value("P04", "P04", false);
        table_0003.add_value("P05", "P05", false);
        table_0003.add_value("P06", "P06", false);
        table_0003.add_value("P07", "P07", false);
        table_0003.add_value("P08", "P08", false);
        table_0003.add_value("P09", "P09", false);
        table_0003.add_value("P10", "P10", false);
        table_0003.add_value("P11", "P11", false);
        table_0003.add_value("P12", "P12", false);
        table_0003.add_value("PC1", "PC1", false);
        table_0003.add_value("PC2", "PC2", false);
        table_0003.add_value("PC3", "PC3", false);
        table_0003.add_value("PC4", "PC4", false);
        table_0003.add_value("PC5", "PC5", false);
        table_0003.add_value("PC6", "PC6", false);
        table_0003.add_value("PC7", "PC7", false);
        table_0003.add_value("PC8", "PC8", false);
        table_0003.add_value("PC9", "PC9", false);
        table_0003.add_value("PCA", "PCA", false);
        table_0003.add_value("PCB", "PCB", false);
        table_0003.add_value("PCC", "PCC", false);
        table_0003.add_value("PCD", "PCD", false);
        table_0003.add_value("PCE", "PCE", false);
        table_0003.add_value("PCF", "PCF", false);
        table_0003.add_value("PCG", "PCG", false);
        table_0003.add_value("PCH", "PCH", false);
        table_0003.add_value("PCJ", "PCJ", false);
        table_0003.add_value("PCK", "PCK", false);
        table_0003.add_value("PCL", "PCL", false);
        table_0003.add_value("Q01", "Q01", false);
        table_0003.add_value("Q02", "Q02", false);
        table_0003.add_value("Q03", "Q03", false);
        table_0003.add_value("Q04", "Q04", false);
        table_0003.add_value("Q05", "Q05", false);
        table_0003.add_value("Q06", "Q06", false);
        table_0003.add_value("Q07", "Q07", false);
        table_0003.add_value("Q08", "Q08", false);
        table_0003.add_value("Q09", "Q09", false);
        table_0003.add_value("Q11", "Q11", false);
        table_0003.add_value("Q13", "Q13", false);
        table_0003.add_value("Q15", "Q15", false);
        table_0003.add_value("Q16", "Q16", false);
        table_0003.add_value("Q17", "Q17", false);
        table_0003.add_value("Q21", "Q21", false);
        table_0003.add_value("Q22", "Q22", false);
        table_0003.add_value("Q23", "Q23", false);
        table_0003.add_value("Q24", "Q24", false);
        table_0003.add_value("Q25", "Q25", false);
        table_0003.add_value("Q26", "Q26", false);
        table_0003.add_value("Q27", "Q27", false);
        table_0003.add_value("Q28", "Q28", false);
        table_0003.add_value("Q29", "Q29", false);
        table_0003.add_value("Q30", "Q30", false);
        table_0003.add_value("Q31", "Q31", false);
        table_0003.add_value("Q32", "Q32", false);
        table_0003.add_value("Q33", "Q33", false);
        table_0003.add_value("Q34", "Q34", false);
        table_0003.add_value("R01", "R01", false);
        table_0003.add_value("R02", "R02", false);
        table_0003.add_value("R03", "R03", false);
        table_0003.add_value("R04", "R04", false);
        table_0003.add_value("R05", "R05", false);
        table_0003.add_value("R06", "R06", false);
        table_0003.add_value("R07", "R07", false);
        table_0003.add_value("R08", "R08", false);
        table_0003.add_value("R09", "R09", false);
        table_0003.add_value("R0R", "R0R", false);
        table_0003.add_value("R21", "R21", false);
        table_0003.add_value("R22", "R22", false);
        table_0003.add_value("R23", "R23", false);
        table_0003.add_value("R24", "R24", false);
        table_0003.add_value("R25", "R25", false);
        table_0003.add_value("R26", "R26", false);
        table_0003.add_value("R30", "R30", false);
        table_0003.add_value("R31", "R31", false);
        table_0003.add_value("R32", "R32", false);
        table_0003.add_value("R33", "R33", false);
        table_0003.add_value("R40", "R40", false);
        table_0003.add_value("R41", "R41", false);
        table_0003.add_value("RAR", "RAR", false);
        table_0003.add_value("RDR", "RDR", false);
        table_0003.add_value("RER", "RER", false);
        table_0003.add_value("RGR", "RGR", false);
        table_0003.add_value("ROR", "ROR", false);
        table_0003.add_value("S01", "S01", false);
        table_0003.add_value("S02", "S02", false);
        table_0003.add_value("S03", "S03", false);
        table_0003.add_value("S04", "S04", false);
        table_0003.add_value("S05", "S05", false);
        table_0003.add_value("S06", "S06", false);
        table_0003.add_value("S07", "S07", false);
        table_0003.add_value("S08", "S08", false);
        table_0003.add_value("S09", "S09", false);
        table_0003.add_value("S10", "S10", false);
        table_0003.add_value("S11", "S11", false);
        table_0003.add_value("S12", "S12", false);
        table_0003.add_value("S13", "S13", false);
        table_0003.add_value("S14", "S14", false);
        table_0003.add_value("S15", "S15", false);
        table_0003.add_value("S16", "S16", false);
        table_0003.add_value("S17", "S17", false);
        table_0003.add_value("S18", "S18", false);
        table_0003.add_value("S19", "S19", false);
        table_0003.add_value("S20", "S20", false);
        table_0003.add_value("S21", "S21", false);
        table_0003.add_value("S22", "S22", false);
        table_0003.add_value("S23", "S23", false);
        table_0003.add_value("S24", "S24", false);
        table_0003.add_value("S25", "S25", false);
        table_0003.add_value("S26", "S26", false);
        table_0003.add_value("S27", "S27", false);
        table_0003.add_value("S28", "S28", false);
        table_0003.add_value("S29", "S29", false);
        table_0003.add_value("S30", "S30", false);
        table_0003.add_value("S31", "S31", false);
        table_0003.add_value("S32", "S32", false);
        table_0003.add_value("S33", "S33", false);
        table_0003.add_value("S34", "S34", false);
        table_0003.add_value("S35", "S35", false);
        table_0003.add_value("S36", "S36", false);
        table_0003.add_value("S37", "S37", false);
        table_0003.add_value("T01", "T01", false);
        table_0003.add_value("T02", "T02", false);
        table_0003.add_value("T03", "T03", false);
        table_0003.add_value("T04", "T04", false);
        table_0003.add_value("T05", "T05", false);
        table_0003.add_value("T06", "T06", false);
        table_0003.add_value("T07", "T07", false);
        table_0003.add_value("T08", "T08", false);
        table_0003.add_value("T09", "T09", false);
        table_0003.add_value("T10", "T10", false);
        table_0003.add_value("T11", "T11", false);
        table_0003.add_value("T12", "T12", false);
        table_0003.add_value("U01", "U01", false);
        table_0003.add_value("U02", "U02", false);
        table_0003.add_value("U03", "U03", false);
        table_0003.add_value("U04", "U04", false);
        table_0003.add_value("U05", "U05", false);
        table_0003.add_value("U06", "U06", false);
        table_0003.add_value("U07", "U07", false);
        table_0003.add_value("U08", "U08", false);
        table_0003.add_value("U09", "U09", false);
        table_0003.add_value("U10", "U10", false);
        table_0003.add_value("U11", "U11", false);
        table_0003.add_value("U12", "U12", false);
        table_0003.add_value("U13", "U13", false);
        table_0003.add_value("V01", "V01", false);
        table_0003.add_value("V02", "V02", false);
        table_0003.add_value("V03", "V03", false);
        table_0003.add_value("V04", "V04", false);
        table_0003.add_value("W01", "W01", false);
        table_0003.add_value("W02", "W02", false);
        table_0003.add_value("Z73", "Z73", false);
        table_0003.add_value("Z74", "Z74", false);
        table_0003.add_value("Z75", "Z75", false);
        table_0003.add_value("Z76", "Z76", false);
        table_0003.add_value("Z77", "Z77", false);
        table_0003.add_value("Z78", "Z78", false);
        table_0003.add_value("Z79", "Z79", false);
        table_0003.add_value("Z80", "Z80", false);
        table_0003.add_value("Z81", "Z81", false);
        table_0003.add_value("Z82", "Z82", false);
        table_0003.add_value("Z83", "Z83", false);
        table_0003.add_value("Z84", "Z84", false);
        table_0003.add_value("Z85", "Z85", false);
        table_0003.add_value("Z86", "Z86", false);
        table_0003.add_value("Z87", "Z87", false);
        table_0003.add_value("Z88", "Z88", false);
        table_0003.add_value("Z89", "Z89", false);
        table_0003.add_value("Z90", "Z90", false);
        table_0003.add_value("Z91", "Z91", false);
        table_0003.add_value("Z92", "Z92", false);
        table_0003.add_value("Z93", "Z93", false);
        table_0003.add_value("Z94", "Z94", false);
        table_0003.add_value("Z95", "Z95", false);
        table_0003.add_value("Z96", "Z96", false);
        table_0003.add_value("Z97", "Z97", false);
        table_0003.add_value("Z98", "Z98", false);
        table_0003.add_value("Z99", "Z99", false);
        table_0003.add_value("E", "Emergency", false);
        table_0003.add_value("I", "Inpatient", false);
        table_0003.add_value("O", "Outpatient", false);
        table_0003.add_value("P", "Preadmit", false);
        table_0003.add_value("R", "Recurring patient", false);
        table_0003.add_value("B", "Obstetrics", false);
        table_0003.add_value("C", "Commercial account", false);
        table_0003.add_value("N", "Not applicable", false);
        table_0003.add_value("U", "Unknown", false);
        self.add_table(table_0003);

        // HL7 Table 0004 - Patient Class
        let mut table_0004 = Hl7Table::new("0004", "Patient Class", "Patient Class");
        table_0004.add_value("B", "B", false);
        table_0004.add_value("C", "C", false);
        table_0004.add_value("E", "E", false);
        table_0004.add_value("I", "I", false);
        table_0004.add_value("N", "N", false);
        table_0004.add_value("O", "O", false);
        table_0004.add_value("P", "P", false);
        table_0004.add_value("R", "R", false);
        table_0004.add_value("U", "U", false);
        self.add_table(table_0004);

        // HL7 Table 0005 - Race
        let mut table_0005 = Hl7Table::new("0005", "Race", "Race");
        table_0005.add_value("1002-5", "1002-5", false);
        table_0005.add_value("2028-9", "2028-9", false);
        table_0005.add_value("2054-5", "2054-5", false);
        table_0005.add_value("2076-8", "2076-8", false);
        table_0005.add_value("2106-3", "2106-3", false);
        table_0005.add_value("2131-1", "2131-1", false);
        self.add_table(table_0005);

        // HL7 Table 0006 - Religion
        let mut table_0006 = Hl7Table::new("0006", "Religion", "Religion");
        table_0006.add_value("ABC", "ABC", false);
        table_0006.add_value("AGN", "AGN", false);
        table_0006.add_value("AME", "AME", false);
        table_0006.add_value("AMT", "AMT", false);
        table_0006.add_value("ANG", "ANG", false);
        table_0006.add_value("AOG", "AOG", false);
        table_0006.add_value("ATH", "ATH", false);
        table_0006.add_value("BAH", "BAH", false);
        table_0006.add_value("BAP", "BAP", false);
        table_0006.add_value("BMA", "BMA", false);
        table_0006.add_value("BOT", "BOT", false);
        table_0006.add_value("BRE", "BRE", false);
        table_0006.add_value("BTA", "BTA", false);
        table_0006.add_value("BTH", "BTH", false);
        table_0006.add_value("BUD", "BUD", false);
        table_0006.add_value("CAT", "CAT", false);
        table_0006.add_value("CFR", "CFR", false);
        table_0006.add_value("CHR", "CHR", false);
        table_0006.add_value("CHS", "CHS", false);
        table_0006.add_value("CMA", "CMA", false);
        table_0006.add_value("CNF", "CNF", false);
        table_0006.add_value("COC", "COC", false);
        table_0006.add_value("COG", "COG", false);
        table_0006.add_value("COI", "COI", false);
        table_0006.add_value("COL", "COL", false);
        table_0006.add_value("COM", "COM", false);
        table_0006.add_value("COP", "COP", false);
        table_0006.add_value("COT", "COT", false);
        table_0006.add_value("CRR", "CRR", false);
        table_0006.add_value("DOC", "DOC", false);
        table_0006.add_value("EOT", "EOT", false);
        table_0006.add_value("EPI", "EPI", false);
        table_0006.add_value("ERL", "ERL", false);
        table_0006.add_value("EVC", "EVC", false);
        table_0006.add_value("FRQ", "FRQ", false);
        table_0006.add_value("FUL", "FUL", false);
        table_0006.add_value("FWB", "FWB", false);
        table_0006.add_value("GRE", "GRE", false);
        table_0006.add_value("HIN", "HIN", false);
        table_0006.add_value("HOT", "HOT", false);
        table_0006.add_value("HSH", "HSH", false);
        table_0006.add_value("HVA", "HVA", false);
        table_0006.add_value("JAI", "JAI", false);
        table_0006.add_value("JCO", "JCO", false);
        table_0006.add_value("JEW", "JEW", false);
        table_0006.add_value("JOR", "JOR", false);
        table_0006.add_value("JOT", "JOT", false);
        table_0006.add_value("JRC", "JRC", false);
        table_0006.add_value("JRF", "JRF", false);
        table_0006.add_value("JRN", "JRN", false);
        table_0006.add_value("JWN", "JWN", false);
        table_0006.add_value("LMS", "LMS", false);
        table_0006.add_value("LUT", "LUT", false);
        table_0006.add_value("MEN", "MEN", false);
        table_0006.add_value("MET", "MET", false);
        table_0006.add_value("MOM", "MOM", false);
        table_0006.add_value("MOS", "MOS", false);
        table_0006.add_value("MOT", "MOT", false);
        table_0006.add_value("MSH", "MSH", false);
        table_0006.add_value("MSU", "MSU", false);
        table_0006.add_value("NAM", "NAM", false);
        table_0006.add_value("NAZ", "NAZ", false);
        table_0006.add_value("NOE", "NOE", false);
        table_0006.add_value("NRL", "NRL", false);
        table_0006.add_value("ORT", "ORT", false);
        table_0006.add_value("OTH", "OTH", false);
        table_0006.add_value("PEN", "PEN", false);
        table_0006.add_value("PRC", "PRC", false);
        table_0006.add_value("PRE", "PRE", false);
        table_0006.add_value("PRO", "PRO", false);
        table_0006.add_value("QUA", "QUA", false);
        table_0006.add_value("REC", "REC", false);
        table_0006.add_value("REO", "REO", false);
        table_0006.add_value("SAA", "SAA", false);
        table_0006.add_value("SEV", "SEV", false);
        table_0006.add_value("SHN", "SHN", false);
        table_0006.add_value("SIK", "SIK", false);
        table_0006.add_value("SOU", "SOU", false);
        table_0006.add_value("SPI", "SPI", false);
        table_0006.add_value("UCC", "UCC", false);
        table_0006.add_value("UMD", "UMD", false);
        table_0006.add_value("UNI", "UNI", false);
        table_0006.add_value("UNU", "UNU", false);
        table_0006.add_value("VAR", "VAR", false);
        table_0006.add_value("WES", "WES", false);
        table_0006.add_value("WMC", "WMC", false);
        self.add_table(table_0006);

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

        // HL7 Table 0008 - Acknowledgment Code
        let mut table_0008 = Hl7Table::new("0008", "Acknowledgment Code", "Acknowledgment Code");
        table_0008.add_value("AA", "AA", false);
        table_0008.add_value("AE", "AE", false);
        table_0008.add_value("AR", "AR", false);
        table_0008.add_value("CA", "CA", false);
        table_0008.add_value("CE", "CE", false);
        table_0008.add_value("CR", "CR", false);
        self.add_table(table_0008);

        // HL7 Table 0009 - Ambulatory Status
        let mut table_0009 = Hl7Table::new("0009", "Ambulatory Status", "Ambulatory Status");
        table_0009.add_value("A0", "A0", false);
        table_0009.add_value("A1", "A1", false);
        table_0009.add_value("A2", "A2", false);
        table_0009.add_value("A3", "A3", false);
        table_0009.add_value("A4", "A4", false);
        table_0009.add_value("A5", "A5", false);
        table_0009.add_value("A6", "A6", false);
        table_0009.add_value("A7", "A7", false);
        table_0009.add_value("A8", "A8", false);
        table_0009.add_value("A9", "A9", false);
        table_0009.add_value("B1", "B1", false);
        table_0009.add_value("B2", "B2", false);
        table_0009.add_value("B3", "B3", false);
        table_0009.add_value("B4", "B4", false);
        table_0009.add_value("B5", "B5", false);
        table_0009.add_value("B6", "B6", false);
        self.add_table(table_0009);

        // HL7 Table 0017 - Transaction Type
        let mut table_0017 = Hl7Table::new("0017", "Transaction Type", "Transaction Type");
        table_0017.add_value("AJ", "AJ", false);
        table_0017.add_value("CD", "CD", false);
        table_0017.add_value("CG", "CG", false);
        table_0017.add_value("CO", "CO", false);
        table_0017.add_value("PY", "PY", false);
        self.add_table(table_0017);

        // HL7 Table 0023 - Admit Source
        let mut table_0023 = Hl7Table::new("0023", "Admit Source", "Admit Source");
        table_0023.add_value("1", "1", false);
        table_0023.add_value("2", "2", false);
        table_0023.add_value("3", "3", false);
        table_0023.add_value("4", "4", false);
        table_0023.add_value("5", "5", false);
        table_0023.add_value("6", "6", false);
        table_0023.add_value("7", "7", false);
        table_0023.add_value("8", "8", false);
        table_0023.add_value("9", "9", false);
        self.add_table(table_0023);

        // HL7 Table 0038 - Order status
        let mut table_0038 = Hl7Table::new("0038", "Order status", "Order status");
        table_0038.add_value("A", "A", false);
        table_0038.add_value("CA", "CA", false);
        table_0038.add_value("CM", "CM", false);
        table_0038.add_value("DC", "DC", false);
        table_0038.add_value("ER", "ER", false);
        table_0038.add_value("HD", "HD", false);
        table_0038.add_value("IP", "IP", false);
        table_0038.add_value("RP", "RP", false);
        table_0038.add_value("SC", "SC", false);
        self.add_table(table_0038);

        // HL7 Table 0043 - Condition Code
        let mut table_0043 = Hl7Table::new("0043", "Condition Code", "Condition Code");
        table_0043.add_value("01", "01", false);
        table_0043.add_value("02", "02", false);
        table_0043.add_value("03", "03", false);
        table_0043.add_value("04", "04", false);
        table_0043.add_value("05", "05", false);
        table_0043.add_value("06", "06", false);
        table_0043.add_value("07", "07", false);
        table_0043.add_value("08", "08", false);
        table_0043.add_value("09", "09", false);
        table_0043.add_value("10", "10", false);
        table_0043.add_value("11", "11", false);
        table_0043.add_value("12", "12", false);
        table_0043.add_value("13", "13", false);
        table_0043.add_value("14", "14", false);
        table_0043.add_value("15", "15", false);
        table_0043.add_value("16", "16", false);
        table_0043.add_value("18", "18", false);
        table_0043.add_value("19", "19", false);
        table_0043.add_value("20", "20", false);
        table_0043.add_value("21", "21", false);
        table_0043.add_value("26", "26", false);
        table_0043.add_value("27", "27", false);
        table_0043.add_value("28", "28", false);
        table_0043.add_value("29", "29", false);
        table_0043.add_value("31", "31", false);
        table_0043.add_value("32", "32", false);
        table_0043.add_value("33", "33", false);
        table_0043.add_value("34", "34", false);
        table_0043.add_value("36", "36", false);
        table_0043.add_value("37", "37", false);
        table_0043.add_value("38", "38", false);
        table_0043.add_value("39", "39", false);
        table_0043.add_value("40", "40", false);
        table_0043.add_value("41", "41", false);
        table_0043.add_value("46", "46", false);
        table_0043.add_value("48", "48", false);
        table_0043.add_value("55", "55", false);
        table_0043.add_value("56", "56", false);
        table_0043.add_value("57", "57", false);
        table_0043.add_value("60", "60", false);
        table_0043.add_value("61", "61", false);
        table_0043.add_value("62", "62", false);
        table_0043.add_value("66", "66", false);
        table_0043.add_value("67", "67", false);
        table_0043.add_value("68", "68", false);
        table_0043.add_value("70", "70", false);
        table_0043.add_value("71", "71", false);
        table_0043.add_value("72", "72", false);
        table_0043.add_value("73", "73", false);
        table_0043.add_value("74", "74", false);
        table_0043.add_value("75", "75", false);
        table_0043.add_value("76", "76", false);
        table_0043.add_value("77", "77", false);
        table_0043.add_value("78", "78", false);
        table_0043.add_value("79", "79", false);
        table_0043.add_value("80", "80", false);
        self.add_table(table_0043);

        // HL7 Table 0048 - What subject filter
        let mut table_0048 = Hl7Table::new("0048", "What subject filter", "What subject filter");
        table_0048.add_value("ADV", "ADV", false);
        table_0048.add_value("ANU", "ANU", false);
        table_0048.add_value("APA", "APA", false);
        table_0048.add_value("APM", "APM", false);
        table_0048.add_value("APN", "APN", false);
        table_0048.add_value("APP", "APP", false);
        table_0048.add_value("ARN", "ARN", false);
        table_0048.add_value("CAN", "CAN", false);
        table_0048.add_value("DEM", "DEM", false);
        table_0048.add_value("FIN", "FIN", false);
        table_0048.add_value("GID", "GID", false);
        table_0048.add_value("GOL", "GOL", false);
        table_0048.add_value("MRI", "MRI", false);
        table_0048.add_value("MRO", "MRO", false);
        table_0048.add_value("NCK", "NCK", false);
        table_0048.add_value("NSC", "NSC", false);
        table_0048.add_value("NST", "NST", false);
        table_0048.add_value("ORD", "ORD", false);
        table_0048.add_value("OTH", "OTH", false);
        table_0048.add_value("PRB", "PRB", false);
        table_0048.add_value("PRO", "PRO", false);
        table_0048.add_value("RAR", "RAR", false);
        table_0048.add_value("RDR", "RDR", false);
        table_0048.add_value("RER", "RER", false);
        table_0048.add_value("RES", "RES", false);
        table_0048.add_value("RGR", "RGR", false);
        table_0048.add_value("ROR", "ROR", false);
        table_0048.add_value("SAL", "SAL", false);
        table_0048.add_value("SBK", "SBK", false);
        table_0048.add_value("SBL", "SBL", false);
        table_0048.add_value("SOF", "SOF", false);
        table_0048.add_value("SOP", "SOP", false);
        table_0048.add_value("SSA", "SSA", false);
        table_0048.add_value("SSR", "SSR", false);
        table_0048.add_value("STA", "STA", false);
        table_0048.add_value("VXI", "VXI", false);
        table_0048.add_value("XID", "XID", false);
        self.add_table(table_0048);

        // HL7 Table 0052 - Diagnosis Type
        let mut table_0052 = Hl7Table::new("0052", "Diagnosis Type", "Diagnosis Type");
        table_0052.add_value("A", "A", false);
        table_0052.add_value("F", "F", false);
        table_0052.add_value("W", "W", false);
        self.add_table(table_0052);

        // HL7 Table 0061 - Check Digit Scheme
        let mut table_0061 =
            Hl7Table::new("0061", "Check Digit Scheme", "Algorithm for check digit");
        table_0061.add_value("M10", "Mod 10 algorithm", false);
        table_0061.add_value("M11", "Mod 11 algorithm", false);
        table_0061.add_value("ISO", "ISO 7064: 1983", false);
        table_0061.add_value("BCV", "BCV", false);
        table_0061.add_value("NPI", "NPI", false);
        self.add_table(table_0061);

        // HL7 Table 0062 - Event Reason
        let mut table_0062 = Hl7Table::new("0062", "Event Reason", "Event Reason");
        table_0062.add_value("01", "01", false);
        table_0062.add_value("02", "02", false);
        table_0062.add_value("03", "03", false);
        table_0062.add_value("1", "1", false);
        table_0062.add_value("2", "2", false);
        table_0062.add_value("3", "3", false);
        table_0062.add_value("O", "O", false);
        table_0062.add_value("U", "U", false);
        self.add_table(table_0062);

        // HL7 Table 0063 - Relationship
        let mut table_0063 =
            Hl7Table::new("0063", "Relationship", "Patient relationship to insured");
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
        table_0063.add_value("ASC", "ASC", false);
        table_0063.add_value("BRO", "BRO", false);
        table_0063.add_value("CGV", "CGV", false);
        table_0063.add_value("DOM", "DOM", false);
        table_0063.add_value("EMC", "EMC", false);
        table_0063.add_value("EXF", "EXF", false);
        table_0063.add_value("FND", "FND", false);
        table_0063.add_value("FTH", "FTH", false);
        table_0063.add_value("GRD", "GRD", false);
        table_0063.add_value("GRP", "GRP", false);
        table_0063.add_value("MGR", "MGR", false);
        table_0063.add_value("MTH", "MTH", false);
        table_0063.add_value("NON", "NON", false);
        table_0063.add_value("OAD", "OAD", false);
        table_0063.add_value("OWN", "OWN", false);
        table_0063.add_value("SIB", "SIB", false);
        table_0063.add_value("SIS", "SIS", false);
        table_0063.add_value("TRA", "TRA", false);
        table_0063.add_value("UNK", "UNK", false);
        table_0063.add_value("WRD", "WRD", false);
        self.add_table(table_0063);

        // HL7 Table 0065 - Specimen Action Code
        let mut table_0065 = Hl7Table::new("0065", "Specimen Action Code", "Specimen Action Code");
        table_0065.add_value("A", "A", false);
        table_0065.add_value("G", "G", false);
        table_0065.add_value("L", "L", false);
        table_0065.add_value("O", "O", false);
        table_0065.add_value("P", "P", false);
        table_0065.add_value("R", "R", false);
        table_0065.add_value("S", "S", false);
        self.add_table(table_0065);

        // HL7 Table 0066 - Employment Status
        let mut table_0066 = Hl7Table::new("0066", "Employment Status", "Employment Status");
        table_0066.add_value("1", "1", false);
        table_0066.add_value("2", "2", false);
        table_0066.add_value("3", "3", false);
        table_0066.add_value("4", "4", false);
        table_0066.add_value("5", "5", false);
        table_0066.add_value("6", "6", false);
        table_0066.add_value("9", "9", false);
        table_0066.add_value("C", "C", false);
        table_0066.add_value("D", "D", false);
        table_0066.add_value("F", "F", false);
        table_0066.add_value("L", "L", false);
        table_0066.add_value("O", "O", false);
        table_0066.add_value("P", "P", false);
        table_0066.add_value("T", "T", false);
        self.add_table(table_0066);

        // HL7 Table 0069 - Hospital Service
        let mut table_0069 = Hl7Table::new("0069", "Hospital Service", "Hospital Service");
        table_0069.add_value("CAR", "CAR", false);
        table_0069.add_value("MED", "MED", false);
        table_0069.add_value("PUL", "PUL", false);
        table_0069.add_value("SUR", "SUR", false);
        table_0069.add_value("URO", "URO", false);
        self.add_table(table_0069);

        // HL7 Table 0070 - Specimen Source Codes
        let mut table_0070 =
            Hl7Table::new("0070", "Specimen Source Codes", "Specimen Source Codes");
        table_0070.add_value("ABS", "ABS", false);
        table_0070.add_value("ACNE", "ACNE", false);
        table_0070.add_value("ACNFLD", "ACNFLD", false);
        table_0070.add_value("AIRS", "AIRS", false);
        table_0070.add_value("ALL", "ALL", false);
        table_0070.add_value("AMN", "AMN", false);
        table_0070.add_value("AMP", "AMP", false);
        table_0070.add_value("ANGI", "ANGI", false);
        table_0070.add_value("ARTC", "ARTC", false);
        table_0070.add_value("ASERU", "ASERU", false);
        table_0070.add_value("ASP", "ASP", false);
        table_0070.add_value("ATTE", "ATTE", false);
        table_0070.add_value("AUTOC", "AUTOC", false);
        table_0070.add_value("AUTP", "AUTP", false);
        table_0070.add_value("BBL", "BBL", false);
        table_0070.add_value("BCYST", "BCYST", false);
        table_0070.add_value("BDY", "BDY", false);
        table_0070.add_value("BIFL", "BIFL", false);
        table_0070.add_value("BITE", "BITE", false);
        table_0070.add_value("BLD", "BLD", false);
        table_0070.add_value("BLDA", "BLDA", false);
        table_0070.add_value("BLDC", "BLDC", false);
        table_0070.add_value("BLDCO", "BLDCO", false);
        table_0070.add_value("BLDV", "BLDV", false);
        table_0070.add_value("BLEB", "BLEB", false);
        table_0070.add_value("BLIST", "BLIST", false);
        table_0070.add_value("BOIL", "BOIL", false);
        table_0070.add_value("BON", "BON", false);
        table_0070.add_value("BOWL", "BOWL", false);
        table_0070.add_value("BPH", "BPH", false);
        table_0070.add_value("BPU", "BPU", false);
        table_0070.add_value("BRN", "BRN", false);
        table_0070.add_value("BRO", "BRO", false);
        table_0070.add_value("BRSH", "BRSH", false);
        table_0070.add_value("BRTH", "BRTH", false);
        table_0070.add_value("BRUS", "BRUS", false);
        table_0070.add_value("BUB", "BUB", false);
        table_0070.add_value("BULLA", "BULLA", false);
        table_0070.add_value("BX", "BX", false);
        table_0070.add_value("CALC", "CALC", false);
        table_0070.add_value("CARBU", "CARBU", false);
        table_0070.add_value("CAT", "CAT", false);
        table_0070.add_value("CBITE", "CBITE", false);
        table_0070.add_value("CBLD", "CBLD", false);
        table_0070.add_value("CDM", "CDM", false);
        table_0070.add_value("CLIPP", "CLIPP", false);
        table_0070.add_value("CNJT", "CNJT", false);
        table_0070.add_value("CNL", "CNL", false);
        table_0070.add_value("COL", "COL", false);
        table_0070.add_value("CONE", "CONE", false);
        table_0070.add_value("CSCR", "CSCR", false);
        table_0070.add_value("CSERU", "CSERU", false);
        table_0070.add_value("CSF", "CSF", false);
        table_0070.add_value("CSITE", "CSITE", false);
        table_0070.add_value("CSMY", "CSMY", false);
        table_0070.add_value("CST", "CST", false);
        table_0070.add_value("CSVR", "CSVR", false);
        table_0070.add_value("CTP", "CTP", false);
        table_0070.add_value("CUR", "CUR", false);
        table_0070.add_value("CVM", "CVM", false);
        table_0070.add_value("CVPS", "CVPS", false);
        table_0070.add_value("CVPT", "CVPT", false);
        table_0070.add_value("CVX", "CVX", false);
        table_0070.add_value("CYN", "CYN", false);
        table_0070.add_value("CYST", "CYST", false);
        table_0070.add_value("DBITE", "DBITE", false);
        table_0070.add_value("DCS", "DCS", false);
        table_0070.add_value("DEC", "DEC", false);
        table_0070.add_value("DEION", "DEION", false);
        table_0070.add_value("DIA", "DIA", false);
        table_0070.add_value("DIAF", "DIAF", false);
        table_0070.add_value("DISCHG", "DISCHG", false);
        table_0070.add_value("DIV", "DIV", false);
        table_0070.add_value("DOSE", "DOSE", false);
        table_0070.add_value("DRN", "DRN", false);
        table_0070.add_value("DRNG", "DRNG", false);
        table_0070.add_value("DRNGP", "DRNGP", false);
        table_0070.add_value("DUFL", "DUFL", false);
        table_0070.add_value("EAR", "EAR", false);
        table_0070.add_value("EARW", "EARW", false);
        table_0070.add_value("EBRUSH", "EBRUSH", false);
        table_0070.add_value("EEYE", "EEYE", false);
        table_0070.add_value("EFF", "EFF", false);
        table_0070.add_value("EFFUS", "EFFUS", false);
        table_0070.add_value("EFOD", "EFOD", false);
        table_0070.add_value("EISO", "EISO", false);
        table_0070.add_value("ELT", "ELT", false);
        table_0070.add_value("ENDC", "ENDC", false);
        table_0070.add_value("ENDM", "ENDM", false);
        table_0070.add_value("ENVIR", "ENVIR", false);
        table_0070.add_value("EOS", "EOS", false);
        table_0070.add_value("EOTH", "EOTH", false);
        table_0070.add_value("ESOI", "ESOI", false);
        table_0070.add_value("ESOS", "ESOS", false);
        table_0070.add_value("ETA", "ETA", false);
        table_0070.add_value("ETTP", "ETTP", false);
        table_0070.add_value("ETTUB", "ETTUB", false);
        table_0070.add_value("EWHI", "EWHI", false);
        table_0070.add_value("EXG", "EXG", false);
        table_0070.add_value("EXHLD", "EXHLD", false);
        table_0070.add_value("EXS", "EXS", false);
        table_0070.add_value("EXUDTE", "EXUDTE", false);
        table_0070.add_value("EYE", "EYE", false);
        table_0070.add_value("FAW", "FAW", false);
        table_0070.add_value("FBLOOD", "FBLOOD", false);
        table_0070.add_value("FGA", "FGA", false);
        table_0070.add_value("FIB", "FIB", false);
        table_0070.add_value("FIST", "FIST", false);
        table_0070.add_value("FLD", "FLD", false);
        table_0070.add_value("FLT", "FLT", false);
        table_0070.add_value("FLU", "FLU", false);
        table_0070.add_value("FLUID", "FLUID", false);
        table_0070.add_value("FOLEY", "FOLEY", false);
        table_0070.add_value("FRS", "FRS", false);
        table_0070.add_value("FSCLP", "FSCLP", false);
        table_0070.add_value("FUR", "FUR", false);
        table_0070.add_value("GAS", "GAS", false);
        table_0070.add_value("GASA", "GASA", false);
        table_0070.add_value("GASAN", "GASAN", false);
        table_0070.add_value("GASBR", "GASBR", false);
        table_0070.add_value("GASD", "GASD", false);
        table_0070.add_value("GAST", "GAST", false);
        table_0070.add_value("GEN", "GEN", false);
        table_0070.add_value("GENC", "GENC", false);
        table_0070.add_value("GENL", "GENL", false);
        table_0070.add_value("GENV", "GENV", false);
        table_0070.add_value("GRAFT", "GRAFT", false);
        table_0070.add_value("GRANU", "GRANU", false);
        table_0070.add_value("GROSH", "GROSH", false);
        table_0070.add_value("GSOL", "GSOL", false);
        table_0070.add_value("GSPEC", "GSPEC", false);
        table_0070.add_value("GT", "GT", false);
        table_0070.add_value("GTUBE", "GTUBE", false);
        table_0070.add_value("HAR", "HAR", false);
        table_0070.add_value("HBITE", "HBITE", false);
        table_0070.add_value("HBLUD", "HBLUD", false);
        table_0070.add_value("HEMAQ", "HEMAQ", false);
        table_0070.add_value("HEMO", "HEMO", false);
        table_0070.add_value("HERNI", "HERNI", false);
        table_0070.add_value("HEV", "HEV", false);
        table_0070.add_value("HIC", "HIC", false);
        table_0070.add_value("HYDC", "HYDC", false);
        table_0070.add_value("IBITE", "IBITE", false);
        table_0070.add_value("ICYST", "ICYST", false);
        table_0070.add_value("IDC", "IDC", false);
        table_0070.add_value("IHG", "IHG", false);
        table_0070.add_value("ILEO", "ILEO", false);
        table_0070.add_value("ILLEG", "ILLEG", false);
        table_0070.add_value("IMP", "IMP", false);
        table_0070.add_value("INCI", "INCI", false);
        table_0070.add_value("INFIL", "INFIL", false);
        table_0070.add_value("INS", "INS", false);
        table_0070.add_value("INTRD", "INTRD", false);
        table_0070.add_value("ISLT", "ISLT", false);
        table_0070.add_value("IT", "IT", false);
        table_0070.add_value("IUD", "IUD", false);
        table_0070.add_value("IVCAT", "IVCAT", false);
        table_0070.add_value("IVFLD", "IVFLD", false);
        table_0070.add_value("IVTIP", "IVTIP", false);
        table_0070.add_value("JEJU", "JEJU", false);
        table_0070.add_value("JNTFLD", "JNTFLD", false);
        table_0070.add_value("JP", "JP", false);
        table_0070.add_value("KELOI", "KELOI", false);
        table_0070.add_value("KIDFLD", "KIDFLD", false);
        table_0070.add_value("LAM", "LAM", false);
        table_0070.add_value("LAVG", "LAVG", false);
        table_0070.add_value("LAVGG", "LAVGG", false);
        table_0070.add_value("LAVGP", "LAVGP", false);
        table_0070.add_value("LAVPG", "LAVPG", false);
        table_0070.add_value("LENS1", "LENS1", false);
        table_0070.add_value("LENS2", "LENS2", false);
        table_0070.add_value("LESN", "LESN", false);
        table_0070.add_value("LIQ", "LIQ", false);
        table_0070.add_value("LIQO", "LIQO", false);
        table_0070.add_value("LN", "LN", false);
        table_0070.add_value("LNA", "LNA", false);
        table_0070.add_value("LNV", "LNV", false);
        table_0070.add_value("LSAC", "LSAC", false);
        table_0070.add_value("LYM", "LYM", false);
        table_0070.add_value("MAC", "MAC", false);
        table_0070.add_value("MAHUR", "MAHUR", false);
        table_0070.add_value("MAR", "MAR", false);
        table_0070.add_value("MASS", "MASS", false);
        table_0070.add_value("MBLD", "MBLD", false);
        table_0070.add_value("MEC", "MEC", false);
        table_0070.add_value("MILK", "MILK", false);
        table_0070.add_value("MLK", "MLK", false);
        table_0070.add_value("MUCOS", "MUCOS", false);
        table_0070.add_value("MUCUS", "MUCUS", false);
        table_0070.add_value("NAIL", "NAIL", false);
        table_0070.add_value("NASDR", "NASDR", false);
        table_0070.add_value("NEDL", "NEDL", false);
        table_0070.add_value("NEPH", "NEPH", false);
        table_0070.add_value("NGASP", "NGASP", false);
        table_0070.add_value("NGAST", "NGAST", false);
        table_0070.add_value("NGS", "NGS", false);
        table_0070.add_value("NODUL", "NODUL", false);
        table_0070.add_value("NOS", "NOS", false);
        table_0070.add_value("NSECR", "NSECR", false);
        table_0070.add_value("ORH", "ORH", false);
        table_0070.add_value("ORL", "ORL", false);
        table_0070.add_value("OTH", "OTH", false);
        table_0070.add_value("PACEM", "PACEM", false);
        table_0070.add_value("PAFL", "PAFL", false);
        table_0070.add_value("PAT", "PAT", false);
        table_0070.add_value("PCFL", "PCFL", false);
        table_0070.add_value("PDSIT", "PDSIT", false);
        table_0070.add_value("PDTS", "PDTS", false);
        table_0070.add_value("PELVA", "PELVA", false);
        table_0070.add_value("PENIL", "PENIL", false);
        table_0070.add_value("PERIA", "PERIA", false);
        table_0070.add_value("PILOC", "PILOC", false);
        table_0070.add_value("PINS", "PINS", false);
        table_0070.add_value("PIS", "PIS", false);
        table_0070.add_value("PLAN", "PLAN", false);
        table_0070.add_value("PLAS", "PLAS", false);
        table_0070.add_value("PLB", "PLB", false);
        table_0070.add_value("PLC", "PLC", false);
        table_0070.add_value("PLEVS", "PLEVS", false);
        table_0070.add_value("PLR", "PLR", false);
        table_0070.add_value("PMN", "PMN", false);
        table_0070.add_value("PND", "PND", false);
        table_0070.add_value("POL", "POL", false);
        table_0070.add_value("POPGS", "POPGS", false);
        table_0070.add_value("POPLG", "POPLG", false);
        table_0070.add_value("POPLV", "POPLV", false);
        table_0070.add_value("PORTA", "PORTA", false);
        table_0070.add_value("PPP", "PPP", false);
        table_0070.add_value("PROST", "PROST", false);
        table_0070.add_value("PRP", "PRP", false);
        table_0070.add_value("PRT", "PRT", false);
        table_0070.add_value("PSC", "PSC", false);
        table_0070.add_value("PUNCT", "PUNCT", false);
        table_0070.add_value("PUS", "PUS", false);
        table_0070.add_value("PUSFR", "PUSFR", false);
        table_0070.add_value("PUST", "PUST", false);
        table_0070.add_value("QC3", "QC3", false);
        table_0070.add_value("RANDU", "RANDU", false);
        table_0070.add_value("RBC", "RBC", false);
        table_0070.add_value("RBITE", "RBITE", false);
        table_0070.add_value("RECT", "RECT", false);
        table_0070.add_value("RECTA", "RECTA", false);
        table_0070.add_value("RENALC", "RENALC", false);
        table_0070.add_value("RENC", "RENC", false);
        table_0070.add_value("RES", "RES", false);
        table_0070.add_value("RT", "RT", false);
        table_0070.add_value("SAL", "SAL", false);
        table_0070.add_value("SCAR", "SCAR", false);
        table_0070.add_value("SCLV", "SCLV", false);
        table_0070.add_value("SCROA", "SCROA", false);
        table_0070.add_value("SECRE", "SECRE", false);
        table_0070.add_value("SEM", "SEM", false);
        table_0070.add_value("SER", "SER", false);
        table_0070.add_value("SHU", "SHU", false);
        table_0070.add_value("SHUNF", "SHUNF", false);
        table_0070.add_value("SHUNT", "SHUNT", false);
        table_0070.add_value("SITE", "SITE", false);
        table_0070.add_value("SKBP", "SKBP", false);
        table_0070.add_value("SKM", "SKM", false);
        table_0070.add_value("SKN", "SKN", false);
        table_0070.add_value("SMM", "SMM", false);
        table_0070.add_value("SMN", "SMN", false);
        table_0070.add_value("SNV", "SNV", false);
        table_0070.add_value("SPRM", "SPRM", false);
        table_0070.add_value("SPRP", "SPRP", false);
        table_0070.add_value("SPRPB", "SPRPB", false);
        table_0070.add_value("SPS", "SPS", false);
        table_0070.add_value("SPT", "SPT", false);
        table_0070.add_value("SPTC", "SPTC", false);
        table_0070.add_value("SPTT", "SPTT", false);
        table_0070.add_value("SPUT1", "SPUT1", false);
        table_0070.add_value("SPUTIN", "SPUTIN", false);
        table_0070.add_value("SPUTSP", "SPUTSP", false);
        table_0070.add_value("STER", "STER", false);
        table_0070.add_value("STL", "STL", false);
        table_0070.add_value("STON", "STON", false);
        table_0070.add_value("STONE", "STONE", false);
        table_0070.add_value("SUBMA", "SUBMA", false);
        table_0070.add_value("SUBMX", "SUBMX", false);
        table_0070.add_value("SUMP", "SUMP", false);
        table_0070.add_value("SUP", "SUP", false);
        table_0070.add_value("SUTUR", "SUTUR", false);
        table_0070.add_value("SWGZ", "SWGZ", false);
        table_0070.add_value("SWT", "SWT", false);
        table_0070.add_value("TASP", "TASP", false);
        table_0070.add_value("TEAR", "TEAR", false);
        table_0070.add_value("THRB", "THRB", false);
        table_0070.add_value("THRT", "THRT", false);
        table_0070.add_value("TISG", "TISG", false);
        table_0070.add_value("TISPL", "TISPL", false);
        table_0070.add_value("TISS", "TISS", false);
        table_0070.add_value("TISU", "TISU", false);
        table_0070.add_value("TLC", "TLC", false);
        table_0070.add_value("TLGI", "TLGI", false);
        table_0070.add_value("TLNG", "TLNG", false);
        table_0070.add_value("TRAC", "TRAC", false);
        table_0070.add_value("TRANS", "TRANS", false);
        table_0070.add_value("TSERU", "TSERU", false);
        table_0070.add_value("TSMI", "TSMI", false);
        table_0070.add_value("TSTES", "TSTES", false);
        table_0070.add_value("TTRA", "TTRA", false);
        table_0070.add_value("TUB", "TUB", false);
        table_0070.add_value("TUBES", "TUBES", false);
        table_0070.add_value("TUMOR", "TUMOR", false);
        table_0070.add_value("TZANC", "TZANC", false);
        table_0070.add_value("UDENT", "UDENT", false);
        table_0070.add_value("ULC", "ULC", false);
        table_0070.add_value("UMB", "UMB", false);
        table_0070.add_value("UMED", "UMED", false);
        table_0070.add_value("UR", "UR", false);
        table_0070.add_value("URC", "URC", false);
        table_0070.add_value("URINB", "URINB", false);
        table_0070.add_value("URINC", "URINC", false);
        table_0070.add_value("URINM", "URINM", false);
        table_0070.add_value("URINN", "URINN", false);
        table_0070.add_value("URINP", "URINP", false);
        table_0070.add_value("URNS", "URNS", false);
        table_0070.add_value("URT", "URT", false);
        table_0070.add_value("URTH", "URTH", false);
        table_0070.add_value("USCOP", "USCOP", false);
        table_0070.add_value("USPEC", "USPEC", false);
        table_0070.add_value("USUB", "USUB", false);
        table_0070.add_value("VASTIP", "VASTIP", false);
        table_0070.add_value("VENT", "VENT", false);
        table_0070.add_value("VITF", "VITF", false);
        table_0070.add_value("VOM", "VOM", false);
        table_0070.add_value("WASH", "WASH", false);
        table_0070.add_value("WASI", "WASI", false);
        table_0070.add_value("WAT", "WAT", false);
        table_0070.add_value("WB", "WB", false);
        table_0070.add_value("WBC", "WBC", false);
        table_0070.add_value("WEN", "WEN", false);
        table_0070.add_value("WICK", "WICK", false);
        table_0070.add_value("WND", "WND", false);
        table_0070.add_value("WNDA", "WNDA", false);
        table_0070.add_value("WNDD", "WNDD", false);
        table_0070.add_value("WNDE", "WNDE", false);
        table_0070.add_value("WORM", "WORM", false);
        table_0070.add_value("WRT", "WRT", false);
        table_0070.add_value("WWA", "WWA", false);
        table_0070.add_value("WWO", "WWO", false);
        table_0070.add_value("WWT", "WWT", false);
        table_0070.add_value("XXX", "XXX", false);
        table_0070.add_value("AU", "AU", false);
        table_0070.add_value("BG", "BG", false);
        table_0070.add_value("BLB", "BLB", false);
        table_0070.add_value("CG", "CG", false);
        table_0070.add_value("CH", "CH", false);
        table_0070.add_value("CP", "CP", false);
        table_0070.add_value("CT", "CT", false);
        table_0070.add_value("CTH", "CTH", false);
        table_0070.add_value("CUS", "CUS", false);
        table_0070.add_value("EC", "EC", false);
        table_0070.add_value("EN", "EN", false);
        table_0070.add_value("GE", "GE", false);
        table_0070.add_value("HM", "HM", false);
        table_0070.add_value("ICU", "ICU", false);
        table_0070.add_value("IMG", "IMG", false);
        table_0070.add_value("IMM", "IMM", false);
        table_0070.add_value("LAB", "LAB", false);
        table_0070.add_value("MB", "MB", false);
        table_0070.add_value("MCB", "MCB", false);
        table_0070.add_value("MYC", "MYC", false);
        table_0070.add_value("NMR", "NMR", false);
        table_0070.add_value("NMS", "NMS", false);
        table_0070.add_value("NRS", "NRS", false);
        table_0070.add_value("OSL", "OSL", false);
        table_0070.add_value("OT", "OT", false);
        table_0070.add_value("OUS", "OUS", false);
        table_0070.add_value("PAR", "PAR", false);
        table_0070.add_value("PF", "PF", false);
        table_0070.add_value("PHR", "PHR", false);
        table_0070.add_value("PHY", "PHY", false);
        table_0070.add_value("PT", "PT", false);
        table_0070.add_value("RAD", "RAD", false);
        table_0070.add_value("RC", "RC", false);
        table_0070.add_value("RUS", "RUS", false);
        table_0070.add_value("RX", "RX", false);
        table_0070.add_value("SP", "SP", false);
        table_0070.add_value("SR", "SR", false);
        table_0070.add_value("TX", "TX", false);
        table_0070.add_value("URN", "URN", false);
        table_0070.add_value("VR", "VR", false);
        table_0070.add_value("VUS", "VUS", false);
        table_0070.add_value("XRC", "XRC", false);
        self.add_table(table_0070);

        // HL7 Table 0074 - Diagnostic Service Section ID
        let mut table_0074 = Hl7Table::new(
            "0074",
            "Diagnostic Service Section ID",
            "Diagnostic Service Section ID",
        );
        table_0074.add_value("AU", "AU", false);
        table_0074.add_value("BG", "BG", false);
        table_0074.add_value("BLB", "BLB", false);
        table_0074.add_value("CG", "CG", false);
        table_0074.add_value("CH", "CH", false);
        table_0074.add_value("CP", "CP", false);
        table_0074.add_value("CT", "CT", false);
        table_0074.add_value("CTH", "CTH", false);
        table_0074.add_value("CUS", "CUS", false);
        table_0074.add_value("EC", "EC", false);
        table_0074.add_value("EN", "EN", false);
        table_0074.add_value("GE", "GE", false);
        table_0074.add_value("HM", "HM", false);
        table_0074.add_value("ICU", "ICU", false);
        table_0074.add_value("IMG", "IMG", false);
        table_0074.add_value("IMM", "IMM", false);
        table_0074.add_value("LAB", "LAB", false);
        table_0074.add_value("MB", "MB", false);
        table_0074.add_value("MCB", "MCB", false);
        table_0074.add_value("MYC", "MYC", false);
        table_0074.add_value("NMR", "NMR", false);
        table_0074.add_value("NMS", "NMS", false);
        table_0074.add_value("NRS", "NRS", false);
        table_0074.add_value("OSL", "OSL", false);
        table_0074.add_value("OT", "OT", false);
        table_0074.add_value("OTH", "OTH", false);
        table_0074.add_value("OUS", "OUS", false);
        table_0074.add_value("PAR", "PAR", false);
        table_0074.add_value("PAT", "PAT", false);
        table_0074.add_value("PF", "PF", false);
        table_0074.add_value("PHR", "PHR", false);
        table_0074.add_value("PHY", "PHY", false);
        table_0074.add_value("PT", "PT", false);
        table_0074.add_value("RAD", "RAD", false);
        table_0074.add_value("RC", "RC", false);
        table_0074.add_value("RT", "RT", false);
        table_0074.add_value("RUS", "RUS", false);
        table_0074.add_value("RX", "RX", false);
        table_0074.add_value("SP", "SP", false);
        table_0074.add_value("SR", "SR", false);
        table_0074.add_value("TX", "TX", false);
        table_0074.add_value("URN", "URN", false);
        table_0074.add_value("VR", "VR", false);
        table_0074.add_value("VUS", "VUS", false);
        table_0074.add_value("XRC", "XRC", false);
        self.add_table(table_0074);

        // HL7 Table 0076 - Message Type
        let mut table_0076 = Hl7Table::new("0076", "Message Type", "Message Type");
        table_0076.add_value("ACK", "ACK", false);
        table_0076.add_value("ADR", "ADR", false);
        table_0076.add_value("ADT", "ADT", false);
        table_0076.add_value("ARD", "ARD", false);
        table_0076.add_value("BAR", "BAR", false);
        table_0076.add_value("BPS", "BPS", false);
        table_0076.add_value("BRP", "BRP", false);
        table_0076.add_value("BRT", "BRT", false);
        table_0076.add_value("BTS", "BTS", false);
        table_0076.add_value("CCF", "CCF", false);
        table_0076.add_value("CCI", "CCI", false);
        table_0076.add_value("CCM", "CCM", false);
        table_0076.add_value("CCQ", "CCQ", false);
        table_0076.add_value("CCU", "CCU", false);
        table_0076.add_value("CQU", "CQU", false);
        table_0076.add_value("CRM", "CRM", false);
        table_0076.add_value("CSU", "CSU", false);
        table_0076.add_value("DBC", "DBC", false);
        table_0076.add_value("DBU", "DBU", false);
        table_0076.add_value("DEL", "DEL", false);
        table_0076.add_value("DEO", "DEO", false);
        table_0076.add_value("DER", "DER", false);
        table_0076.add_value("DFT", "DFT", false);
        table_0076.add_value("DOC", "DOC", false);
        table_0076.add_value("DPR", "DPR", false);
        table_0076.add_value("DRC", "DRC", false);
        table_0076.add_value("DSR", "DSR", false);
        table_0076.add_value("EAC", "EAC", false);
        table_0076.add_value("EAN", "EAN", false);
        table_0076.add_value("EAR", "EAR", false);
        table_0076.add_value("EDR", "EDR", false);
        table_0076.add_value("EHC", "EHC", false);
        table_0076.add_value("EQQ", "EQQ", false);
        table_0076.add_value("ERP", "ERP", false);
        table_0076.add_value("ESR", "ESR", false);
        table_0076.add_value("ESU", "ESU", false);
        table_0076.add_value("INR", "INR", false);
        table_0076.add_value("INU", "INU", false);
        table_0076.add_value("LSR", "LSR", false);
        table_0076.add_value("LSU", "LSU", false);
        table_0076.add_value("MCF", "MCF", false);
        table_0076.add_value("MDM", "MDM", false);
        table_0076.add_value("MFD", "MFD", false);
        table_0076.add_value("MFK", "MFK", false);
        table_0076.add_value("MFN", "MFN", false);
        table_0076.add_value("MFQ", "MFQ", false);
        table_0076.add_value("MFR", "MFR", false);
        table_0076.add_value("NMD", "NMD", false);
        table_0076.add_value("NMQ", "NMQ", false);
        table_0076.add_value("NMR", "NMR", false);
        table_0076.add_value("OMB", "OMB", false);
        table_0076.add_value("OMD", "OMD", false);
        table_0076.add_value("OMG", "OMG", false);
        table_0076.add_value("OMI", "OMI", false);
        table_0076.add_value("OML", "OML", false);
        table_0076.add_value("OMN", "OMN", false);
        table_0076.add_value("OMP", "OMP", false);
        table_0076.add_value("OMQ", "OMQ", false);
        table_0076.add_value("OMS", "OMS", false);
        table_0076.add_value("OPL", "OPL", false);
        table_0076.add_value("OPR", "OPR", false);
        table_0076.add_value("OPU", "OPU", false);
        table_0076.add_value("ORA", "ORA", false);
        table_0076.add_value("ORB", "ORB", false);
        table_0076.add_value("ORD", "ORD", false);
        table_0076.add_value("ORF", "ORF", false);
        table_0076.add_value("ORG", "ORG", false);
        table_0076.add_value("ORI", "ORI", false);
        table_0076.add_value("ORL", "ORL", false);
        table_0076.add_value("ORM", "ORM", false);
        table_0076.add_value("ORN", "ORN", false);
        table_0076.add_value("ORP", "ORP", false);
        table_0076.add_value("ORR", "ORR", false);
        table_0076.add_value("ORS", "ORS", false);
        table_0076.add_value("ORU", "ORU", false);
        table_0076.add_value("ORX", "ORX", false);
        table_0076.add_value("OSM", "OSM", false);
        table_0076.add_value("OSQ", "OSQ", false);
        table_0076.add_value("OSR", "OSR", false);
        table_0076.add_value("OSU", "OSU", false);
        table_0076.add_value("OUL", "OUL", false);
        table_0076.add_value("PEX", "PEX", false);
        table_0076.add_value("PGL", "PGL", false);
        table_0076.add_value("PIN", "PIN", false);
        table_0076.add_value("PMU", "PMU", false);
        table_0076.add_value("PPG", "PPG", false);
        table_0076.add_value("PPP", "PPP", false);
        table_0076.add_value("PPR", "PPR", false);
        table_0076.add_value("PPT", "PPT", false);
        table_0076.add_value("PPV", "PPV", false);
        table_0076.add_value("PRR", "PRR", false);
        table_0076.add_value("PTR", "PTR", false);
        table_0076.add_value("QBP", "QBP", false);
        table_0076.add_value("QCK", "QCK", false);
        table_0076.add_value("QCN", "QCN", false);
        table_0076.add_value("QRY", "QRY", false);
        table_0076.add_value("QSB", "QSB", false);
        table_0076.add_value("QSX", "QSX", false);
        table_0076.add_value("QVR", "QVR", false);
        table_0076.add_value("RAR", "RAR", false);
        table_0076.add_value("RAS", "RAS", false);
        table_0076.add_value("RCI", "RCI", false);
        table_0076.add_value("RCL", "RCL", false);
        table_0076.add_value("RDE", "RDE", false);
        table_0076.add_value("RDR", "RDR", false);
        table_0076.add_value("RDS", "RDS", false);
        table_0076.add_value("RDY", "RDY", false);
        table_0076.add_value("REF", "REF", false);
        table_0076.add_value("RER", "RER", false);
        table_0076.add_value("RGR", "RGR", false);
        table_0076.add_value("RGV", "RGV", false);
        table_0076.add_value("ROR", "ROR", false);
        table_0076.add_value("RPA", "RPA", false);
        table_0076.add_value("RPI", "RPI", false);
        table_0076.add_value("RPL", "RPL", false);
        table_0076.add_value("RPR", "RPR", false);
        table_0076.add_value("RQA", "RQA", false);
        table_0076.add_value("RQC", "RQC", false);
        table_0076.add_value("RQI", "RQI", false);
        table_0076.add_value("RQP", "RQP", false);
        table_0076.add_value("RQQ", "RQQ", false);
        table_0076.add_value("RRA", "RRA", false);
        table_0076.add_value("RRD", "RRD", false);
        table_0076.add_value("RRE", "RRE", false);
        table_0076.add_value("RRG", "RRG", false);
        table_0076.add_value("RRI", "RRI", false);
        table_0076.add_value("RSP", "RSP", false);
        table_0076.add_value("RTB", "RTB", false);
        table_0076.add_value("SCN", "SCN", false);
        table_0076.add_value("SDN", "SDN", false);
        table_0076.add_value("SDR", "SDR", false);
        table_0076.add_value("SIU", "SIU", false);
        table_0076.add_value("SLN", "SLN", false);
        table_0076.add_value("SLR", "SLR", false);
        table_0076.add_value("SMD", "SMD", false);
        table_0076.add_value("SPQ", "SPQ", false);
        table_0076.add_value("SQM", "SQM", false);
        table_0076.add_value("SQR", "SQR", false);
        table_0076.add_value("SRM", "SRM", false);
        table_0076.add_value("SRR", "SRR", false);
        table_0076.add_value("SSR", "SSR", false);
        table_0076.add_value("SSU", "SSU", false);
        table_0076.add_value("STC", "STC", false);
        table_0076.add_value("STI", "STI", false);
        table_0076.add_value("SUR", "SUR", false);
        table_0076.add_value("TBR", "TBR", false);
        table_0076.add_value("TCR", "TCR", false);
        table_0076.add_value("TCU", "TCU", false);
        table_0076.add_value("UDM", "UDM", false);
        table_0076.add_value("VQQ", "VQQ", false);
        table_0076.add_value("VXQ", "VXQ", false);
        table_0076.add_value("VXR", "VXR", false);
        table_0076.add_value("VXU", "VXU", false);
        table_0076.add_value("VXX", "VXX", false);
        self.add_table(table_0076);

        // HL7 Table 0078 - Interpretation Codes
        let mut table_0078 =
            Hl7Table::new("0078", "Interpretation Codes", "Observation interpretation");
        table_0078.add_value("L", "Below low normal", false);
        table_0078.add_value("H", "Above high normal", false);
        table_0078.add_value("LL", "Below lower panic limits", false);
        table_0078.add_value("HH", "Above upper panic limits", false);
        table_0078.add_value("<", "Below absolute low-off instrument scale", false);
        table_0078.add_value(">", "Above absolute high-off instrument scale", false);
        table_0078.add_value("N", "N", false);
        table_0078.add_value("A", "A", false);
        table_0078.add_value("AA", "Very abnormal", false);
        table_0078.add_value("S", "S", false);
        table_0078.add_value("R", "R", false);
        table_0078.add_value("I", "Intermediate", false);
        table_0078.add_value("MS", "Moderately susceptible", false);
        table_0078.add_value("VS", "Very susceptible", false);
        table_0078.add_value("AC", "AC", false);
        table_0078.add_value("B", "B", false);
        table_0078.add_value("D", "D", false);
        table_0078.add_value("DET", "DET", false);
        table_0078.add_value("HM", "HM", false);
        table_0078.add_value("HU", "HU", false);
        table_0078.add_value("IE", "IE", false);
        table_0078.add_value("IND", "IND", false);
        table_0078.add_value("LU", "LU", false);
        table_0078.add_value("ND", "ND", false);
        table_0078.add_value("NEG", "NEG", false);
        table_0078.add_value("NR", "NR", false);
        table_0078.add_value("NS", "NS", false);
        table_0078.add_value("OBX", "OBX", false);
        table_0078.add_value("POS", "POS", false);
        table_0078.add_value("QCF", "QCF", false);
        table_0078.add_value("RR", "RR", false);
        table_0078.add_value("SDD", "SDD", false);
        table_0078.add_value("SYN-R", "SYN-R", false);
        table_0078.add_value("SYN-S", "SYN-S", false);
        table_0078.add_value("TOX", "TOX", false);
        table_0078.add_value("U", "U", false);
        table_0078.add_value("W", "W", false);
        table_0078.add_value("WR", "WR", false);
        table_0078.add_value("SP", "SP", false);
        table_0078.add_value("ST", "ST", false);
        self.add_table(table_0078);

        // HL7 Table 0080 - Nature of Abnormal Testing
        let mut table_0080 = Hl7Table::new(
            "0080",
            "Nature of Abnormal Testing",
            "Nature of Abnormal Testing",
        );
        table_0080.add_value("A", "A", false);
        table_0080.add_value("B", "B", false);
        table_0080.add_value("N", "N", false);
        table_0080.add_value("R", "R", false);
        table_0080.add_value("S", "S", false);
        table_0080.add_value("SP", "SP", false);
        table_0080.add_value("ST", "ST", false);
        self.add_table(table_0080);

        // HL7 Table 0083 - Outlier Type
        let mut table_0083 = Hl7Table::new("0083", "Outlier Type", "Outlier Type");
        table_0083.add_value("C", "Record coming over is a correction", false);
        table_0083.add_value("D", "Deletes the OBX record", false);
        table_0083.add_value("F", "Final results", false);
        table_0083.add_value("I", "Specimen in lab; results pending", false);
        table_0083.add_value("N", "Not asked", false);
        table_0083.add_value("O", "Order detail description only", false);
        table_0083.add_value("P", "Preliminary results", false);
        table_0083.add_value("R", "Results entered - not verified", false);
        table_0083.add_value("S", "Partial results", false);
        table_0083.add_value("U", "Results status change to final", false);
        table_0083.add_value("W", "Post original as wrong", false);
        table_0083.add_value("X", "Results cannot be obtained", false);
        table_0083.add_value("A", "A", false);
        table_0083.add_value("B", "B", false);
        table_0083.add_value("V", "V", false);
        self.add_table(table_0083);

        // HL7 Table 0085 - Observation Result Status Codes Interpretation
        let mut table_0085 = Hl7Table::new(
            "0085",
            "Observation Result Status Codes Interpretation",
            "Observation Result Status Codes Interpretation",
        );
        table_0085.add_value("A", "A", false);
        table_0085.add_value("B", "B", false);
        table_0085.add_value("C", "C", false);
        table_0085.add_value("D", "D", false);
        table_0085.add_value("F", "F", false);
        table_0085.add_value("I", "I", false);
        table_0085.add_value("N", "N", false);
        table_0085.add_value("O", "O", false);
        table_0085.add_value("P", "P", false);
        table_0085.add_value("R", "R", false);
        table_0085.add_value("S", "S", false);
        table_0085.add_value("U", "U", false);
        table_0085.add_value("V", "V", false);
        table_0085.add_value("W", "W", false);
        table_0085.add_value("X", "X", false);
        self.add_table(table_0085);

        // HL7 Table 0088 - Procedure Code
        let mut table_0088 = Hl7Table::new("0088", "Procedure Code", "Procedure Code");
        table_0088.add_value("C4", "C4", false);
        table_0088.add_value("C5", "C5", false);
        table_0088.add_value("HCPCS", "HCPCS", false);
        table_0088.add_value("HPC", "HPC", false);
        table_0088.add_value("I10P", "I10P", false);
        table_0088.add_value("SCT", "SCT", false);
        self.add_table(table_0088);

        // HL7 Table 0091 - Query Priority
        let mut table_0091 = Hl7Table::new("0091", "Query Priority", "Query Priority");
        table_0091.add_value("D", "D", false);
        table_0091.add_value("I", "I", false);
        self.add_table(table_0091);

        // HL7 Table 0092 - Re-Admission Indicator
        let mut table_0092 =
            Hl7Table::new("0092", "Re-Admission Indicator", "Re-Admission Indicator");
        table_0092.add_value("R", "R", false);
        self.add_table(table_0092);

        // HL7 Table 0093 - Release Information
        let mut table_0093 = Hl7Table::new("0093", "Release Information", "Release Information");
        table_0093.add_value("N", "N", false);
        table_0093.add_value("Y", "Y", false);
        table_0093.add_value("_", "_", false);
        self.add_table(table_0093);

        // HL7 Table 0098 - Type of Agreement
        let mut table_0098 = Hl7Table::new("0098", "Type of Agreement", "Type of Agreement");
        table_0098.add_value("M", "M", false);
        table_0098.add_value("S", "S", false);
        table_0098.add_value("U", "U", false);
        self.add_table(table_0098);

        // HL7 Table 0100 - Invocation event
        let mut table_0100 = Hl7Table::new("0100", "Invocation event", "Invocation event");
        table_0100.add_value("D", "D", false);
        table_0100.add_value("O", "O", false);
        table_0100.add_value("R", "R", false);
        table_0100.add_value("S", "S", false);
        table_0100.add_value("T", "T", false);
        table_0100.add_value("F", "F", false);
        self.add_table(table_0100);

        // HL7 Table 0102 - Delayed acknowledgment type
        let mut table_0102 = Hl7Table::new(
            "0102",
            "Delayed acknowledgment type",
            "Delayed acknowledgment type",
        );
        table_0102.add_value("D", "D", false);
        table_0102.add_value("F", "F", false);
        self.add_table(table_0102);

        // HL7 Table 0103 - Processing ID
        let mut table_0103 = Hl7Table::new("0103", "Processing ID", "Message processing mode");
        table_0103.add_value("D", "Debugging", false);
        table_0103.add_value("P", "Production", false);
        table_0103.add_value("T", "Training", false);
        self.add_table(table_0103);

        // HL7 Table 0104 - Version ID
        let mut table_0104 = Hl7Table::new("0104", "Version ID", "Version ID");
        table_0104.add_value("2", "2", false);
        table_0104.add_value("2.0", "2.0", false);
        table_0104.add_value("2.0D", "2.0D", false);
        table_0104.add_value("2.1", "2.1", false);
        table_0104.add_value("2.2", "2.2", false);
        table_0104.add_value("2.3", "2.3", false);
        table_0104.add_value("2.3.1", "2.3.1", false);
        table_0104.add_value("2.3.2", "2.3.2", false);
        table_0104.add_value("2.4", "2.4", false);
        table_0104.add_value("2.5", "2.5", false);
        table_0104.add_value("2.5.1", "2.5.1", false);
        table_0104.add_value("2.6", "2.6", false);
        table_0104.add_value("2.7", "2.7", false);
        table_0104.add_value("2.7.1", "2.7.1", false);
        table_0104.add_value("2.8", "2.8", false);
        table_0104.add_value("2.8.1", "2.8.1", false);
        self.add_table(table_0104);

        // HL7 Table 0105 - Source of Comment
        let mut table_0105 = Hl7Table::new("0105", "Source of Comment", "Source of Comment");
        table_0105.add_value("L", "L", false);
        table_0105.add_value("O", "O", false);
        table_0105.add_value("P", "P", false);
        table_0105.add_value("D", "D", false);
        table_0105.add_value("R", "R", false);
        table_0105.add_value("T", "T", false);
        self.add_table(table_0105);

        // HL7 Table 0106 - Query/response format code
        let mut table_0106 = Hl7Table::new(
            "0106",
            "Query/response format code",
            "Query/response format code",
        );
        table_0106.add_value("D", "D", false);
        table_0106.add_value("R", "R", false);
        table_0106.add_value("T", "T", false);
        self.add_table(table_0106);

        // HL7 Table 0107 - Deferred response type
        let mut table_0107 =
            Hl7Table::new("0107", "Deferred response type", "Deferred response type");
        table_0107.add_value("B", "B", false);
        table_0107.add_value("L", "L", false);
        self.add_table(table_0107);

        // HL7 Table 0108 - Query results level
        let mut table_0108 = Hl7Table::new("0108", "Query results level", "Query results level");
        table_0108.add_value("O", "O", false);
        table_0108.add_value("R", "R", false);
        table_0108.add_value("S", "S", false);
        table_0108.add_value("T", "T", false);
        self.add_table(table_0108);

        // HL7 Table 0112 - Discharge Disposition
        let mut table_0112 =
            Hl7Table::new("0112", "Discharge Disposition", "Discharge Disposition");
        table_0112.add_value("01", "01", false);
        table_0112.add_value("02", "02", false);
        table_0112.add_value("03", "03", false);
        table_0112.add_value("04", "04", false);
        table_0112.add_value("05", "05", false);
        table_0112.add_value("06", "06", false);
        table_0112.add_value("07", "07", false);
        table_0112.add_value("08", "08", false);
        table_0112.add_value("09", "09", false);
        table_0112.add_value("10", "10", false);
        table_0112.add_value("11", "11", false);
        table_0112.add_value("12", "12", false);
        table_0112.add_value("13", "13", false);
        table_0112.add_value("14", "14", false);
        table_0112.add_value("15", "15", false);
        table_0112.add_value("16", "16", false);
        table_0112.add_value("17", "17", false);
        table_0112.add_value("18", "18", false);
        table_0112.add_value("19", "19", false);
        table_0112.add_value("20", "20", false);
        table_0112.add_value("21", "21", false);
        table_0112.add_value("22", "22", false);
        table_0112.add_value("23", "23", false);
        table_0112.add_value("24", "24", false);
        table_0112.add_value("25", "25", false);
        table_0112.add_value("26", "26", false);
        table_0112.add_value("27", "27", false);
        table_0112.add_value("28", "28", false);
        table_0112.add_value("29", "29", false);
        table_0112.add_value("30", "30", false);
        table_0112.add_value("31", "31", false);
        table_0112.add_value("32", "32", false);
        table_0112.add_value("33", "33", false);
        table_0112.add_value("34", "34", false);
        table_0112.add_value("35", "35", false);
        table_0112.add_value("36", "36", false);
        table_0112.add_value("37", "37", false);
        table_0112.add_value("38", "38", false);
        table_0112.add_value("39", "39", false);
        table_0112.add_value("40", "40", false);
        table_0112.add_value("41", "41", false);
        table_0112.add_value("42", "42", false);
        self.add_table(table_0112);

        // HL7 Table 0116 - Bed Status
        let mut table_0116 = Hl7Table::new("0116", "Bed Status", "Bed Status");
        table_0116.add_value("C", "C", false);
        table_0116.add_value("H", "H", false);
        table_0116.add_value("I", "I", false);
        table_0116.add_value("K", "K", false);
        table_0116.add_value("O", "O", false);
        table_0116.add_value("U", "U", false);
        self.add_table(table_0116);

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
        table_0119.add_value("AF", "AF", false);
        table_0119.add_value("CP", "CP", false);
        table_0119.add_value("DF", "DF", false);
        table_0119.add_value("FU", "FU", false);
        table_0119.add_value("HR", "HR", false);
        table_0119.add_value("LI", "LI", false);
        table_0119.add_value("MC", "MC", false);
        table_0119.add_value("OF", "OF", false);
        table_0119.add_value("OP", "OP", false);
        table_0119.add_value("PR", "PR", false);
        table_0119.add_value("PY", "PY", false);
        table_0119.add_value("RF", "RF", false);
        table_0119.add_value("UF", "UF", false);
        table_0119.add_value("UN", "UN", false);
        self.add_table(table_0119);

        // HL7 Table 0121 - Response Flag
        let mut table_0121 = Hl7Table::new("0121", "Response Flag", "Response Flag");
        table_0121.add_value("D", "D", false);
        table_0121.add_value("E", "E", false);
        table_0121.add_value("F", "F", false);
        table_0121.add_value("N", "N", false);
        table_0121.add_value("R", "R", false);
        self.add_table(table_0121);

        // HL7 Table 0122 - Charge Type
        let mut table_0122 = Hl7Table::new("0122", "Charge Type", "Charge Type");
        table_0122.add_value("CH", "CH", false);
        table_0122.add_value("CO", "CO", false);
        table_0122.add_value("CR", "CR", false);
        table_0122.add_value("DP", "DP", false);
        table_0122.add_value("GR", "GR", false);
        table_0122.add_value("NC", "NC", false);
        table_0122.add_value("PC", "PC", false);
        table_0122.add_value("RS", "RS", false);
        self.add_table(table_0122);

        // HL7 Table 0123 - Result Status
        let mut table_0123 = Hl7Table::new("0123", "Result Status", "Result Status");
        table_0123.add_value("A", "A", false);
        table_0123.add_value("C", "C", false);
        table_0123.add_value("F", "F", false);
        table_0123.add_value("I", "I", false);
        table_0123.add_value("O", "O", false);
        table_0123.add_value("P", "P", false);
        table_0123.add_value("R", "R", false);
        table_0123.add_value("S", "S", false);
        table_0123.add_value("X", "X", false);
        table_0123.add_value("Y", "Y", false);
        table_0123.add_value("Z", "Z", false);
        self.add_table(table_0123);

        // HL7 Table 0124 - Transportation Mode
        let mut table_0124 = Hl7Table::new("0124", "Transportation Mode", "Transportation Mode");
        table_0124.add_value("CART", "CART", false);
        table_0124.add_value("PORT", "PORT", false);
        table_0124.add_value("WALK", "WALK", false);
        table_0124.add_value("WHLC", "WHLC", false);
        self.add_table(table_0124);

        // HL7 Table 0125 - Value Type
        let mut table_0125 = Hl7Table::new("0125", "Value Type", "Value Type");
        table_0125.add_value("AD", "AD", false);
        table_0125.add_value("AUI", "AUI", false);
        table_0125.add_value("CCD", "CCD", false);
        table_0125.add_value("CCP", "CCP", false);
        table_0125.add_value("CD", "CD", false);
        table_0125.add_value("CE", "CE", false);
        table_0125.add_value("CF", "CF", false);
        table_0125.add_value("CK", "CK", false);
        table_0125.add_value("CN", "CN", false);
        table_0125.add_value("CNE", "CNE", false);
        table_0125.add_value("CNN", "CNN", false);
        table_0125.add_value("CP", "CP", false);
        table_0125.add_value("CSU", "CSU", false);
        table_0125.add_value("CWE", "CWE", false);
        table_0125.add_value("CX", "CX", false);
        table_0125.add_value("DDI", "DDI", false);
        table_0125.add_value("DIN", "DIN", false);
        table_0125.add_value("DLD", "DLD", false);
        table_0125.add_value("DLN", "DLN", false);
        table_0125.add_value("DLT", "DLT", false);
        table_0125.add_value("DR", "DR", false);
        table_0125.add_value("DT", "DT", false);
        table_0125.add_value("DTM", "DTM", false);
        table_0125.add_value("DTN", "DTN", false);
        table_0125.add_value("ED", "ED", false);
        table_0125.add_value("EI", "EI", false);
        table_0125.add_value("EIP", "EIP", false);
        table_0125.add_value("ERL", "ERL", false);
        table_0125.add_value("FC", "FC", false);
        table_0125.add_value("FT", "FT", false);
        table_0125.add_value("GTS", "GTS", false);
        table_0125.add_value("HD", "HD", false);
        table_0125.add_value("ICD", "ICD", false);
        table_0125.add_value("ID", "ID", false);
        table_0125.add_value("IS", "IS", false);
        table_0125.add_value("JCC", "JCC", false);
        table_0125.add_value("LA1", "LA1", false);
        table_0125.add_value("LA2", "LA2", false);
        table_0125.add_value("MA", "MA", false);
        table_0125.add_value("MO", "MO", false);
        table_0125.add_value("MOC", "MOC", false);
        table_0125.add_value("MOP", "MOP", false);
        table_0125.add_value("MSG", "MSG", false);
        table_0125.add_value("NA", "NA", false);
        table_0125.add_value("NDL", "NDL", false);
        table_0125.add_value("NM", "NM", false);
        table_0125.add_value("NR", "NR", false);
        table_0125.add_value("OCD", "OCD", false);
        table_0125.add_value("OSP", "OSP", false);
        table_0125.add_value("PIP", "PIP", false);
        table_0125.add_value("PL", "PL", false);
        table_0125.add_value("PLN", "PLN", false);
        table_0125.add_value("PN", "PN", false);
        table_0125.add_value("PPN", "PPN", false);
        table_0125.add_value("PRL", "PRL", false);
        table_0125.add_value("PT", "PT", false);
        table_0125.add_value("PTA", "PTA", false);
        table_0125.add_value("QIP", "QIP", false);
        table_0125.add_value("QSC", "QSC", false);
        table_0125.add_value("RCD", "RCD", false);
        table_0125.add_value("RFR", "RFR", false);
        table_0125.add_value("RI", "RI", false);
        table_0125.add_value("RMC", "RMC", false);
        table_0125.add_value("RP", "RP", false);
        table_0125.add_value("RPT", "RPT", false);
        table_0125.add_value("SCV", "SCV", false);
        table_0125.add_value("SN", "SN", false);
        table_0125.add_value("SNM", "SNM", false);
        table_0125.add_value("SPD", "SPD", false);
        table_0125.add_value("SRT", "SRT", false);
        table_0125.add_value("ST", "ST", false);
        table_0125.add_value("TM", "TM", false);
        table_0125.add_value("TN", "TN", false);
        table_0125.add_value("TS", "TS", false);
        table_0125.add_value("TX", "TX", false);
        table_0125.add_value("UVC", "UVC", false);
        table_0125.add_value("VH", "VH", false);
        table_0125.add_value("VID", "VID", false);
        table_0125.add_value("VR", "VR", false);
        table_0125.add_value("WVI", "WVI", false);
        table_0125.add_value("WVS", "WVS", false);
        table_0125.add_value("XAD", "XAD", false);
        table_0125.add_value("XCN", "XCN", false);
        table_0125.add_value("XON", "XON", false);
        table_0125.add_value("XPN", "XPN", false);
        table_0125.add_value("XTN", "XTN", false);
        table_0125.add_value("CH", "CH", false);
        table_0125.add_value("LI", "LI", false);
        table_0125.add_value("PG", "PG", false);
        table_0125.add_value("RD", "RD", false);
        table_0125.add_value("ZO", "ZO", false);
        self.add_table(table_0125);

        // HL7 Table 0126 - Quantity Limited Request
        let mut table_0126 = Hl7Table::new(
            "0126",
            "Quantity Limited Request",
            "Quantity Limited Request",
        );
        table_0126.add_value("CH", "CH", false);
        table_0126.add_value("LI", "LI", false);
        table_0126.add_value("PG", "PG", false);
        table_0126.add_value("RD", "RD", false);
        table_0126.add_value("ZO", "ZO", false);
        self.add_table(table_0126);

        // HL7 Table 0127 - Allergen Type
        let mut table_0127 = Hl7Table::new("0127", "Allergen Type", "Allergen Type");
        table_0127.add_value("AA", "AA", false);
        table_0127.add_value("DA", "DA", false);
        table_0127.add_value("EA", "EA", false);
        table_0127.add_value("FA", "FA", false);
        table_0127.add_value("LA", "LA", false);
        table_0127.add_value("MA", "MA", false);
        table_0127.add_value("MC", "MC", false);
        table_0127.add_value("PA", "PA", false);
        self.add_table(table_0127);

        // HL7 Table 0128 - Allergy Severity
        let mut table_0128 = Hl7Table::new("0128", "Allergy Severity", "Allergy Severity");
        table_0128.add_value("MI", "MI", false);
        table_0128.add_value("MO", "MO", false);
        table_0128.add_value("SV", "SV", false);
        table_0128.add_value("U", "U", false);
        self.add_table(table_0128);

        // HL7 Table 0130 - Visit User Code
        let mut table_0130 = Hl7Table::new("0130", "Visit User Code", "Visit User Code");
        table_0130.add_value("HO", "HO", false);
        table_0130.add_value("MO", "MO", false);
        table_0130.add_value("PH", "PH", false);
        table_0130.add_value("TE", "TE", false);
        self.add_table(table_0130);

        // HL7 Table 0131 - Contact Role
        let mut table_0131 = Hl7Table::new("0131", "Contact Role", "Contact Role");
        table_0131.add_value("C", "C", false);
        table_0131.add_value("E", "E", false);
        table_0131.add_value("F", "F", false);
        table_0131.add_value("I", "I", false);
        table_0131.add_value("N", "N", false);
        table_0131.add_value("O", "O", false);
        table_0131.add_value("S", "S", false);
        table_0131.add_value("U", "U", false);
        self.add_table(table_0131);

        // HL7 Table 0135 - Assignment of Benefits
        let mut table_0135 =
            Hl7Table::new("0135", "Assignment of Benefits", "Assignment of Benefits");
        table_0135.add_value("M", "M", false);
        table_0135.add_value("N", "N", false);
        table_0135.add_value("Y", "Y", false);
        self.add_table(table_0135);

        // HL7 Table 0136 - Yes/no Indicator
        let mut table_0136 = Hl7Table::new("0136", "Yes/no Indicator", "Yes/no Indicator");
        table_0136.add_value("N", "N", false);
        table_0136.add_value("Y", "Y", false);
        self.add_table(table_0136);

        // HL7 Table 0137 - Mail Claim Party
        let mut table_0137 = Hl7Table::new("0137", "Mail Claim Party", "Mail Claim Party");
        table_0137.add_value("E", "E", false);
        table_0137.add_value("G", "G", false);
        table_0137.add_value("I", "I", false);
        table_0137.add_value("O", "O", false);
        table_0137.add_value("P", "P", false);
        self.add_table(table_0137);

        // HL7 Table 0140 - Military Service
        let mut table_0140 = Hl7Table::new("0140", "Military Service", "Military Service");
        table_0140.add_value("AUSA", "AUSA", false);
        table_0140.add_value("AUSAF", "AUSAF", false);
        table_0140.add_value("AUSN", "AUSN", false);
        table_0140.add_value("NATO", "NATO", false);
        table_0140.add_value("NOAA", "NOAA", false);
        table_0140.add_value("USA", "USA", false);
        table_0140.add_value("USAF", "USAF", false);
        table_0140.add_value("USCG", "USCG", false);
        table_0140.add_value("USMC", "USMC", false);
        table_0140.add_value("USN", "USN", false);
        table_0140.add_value("USPHS", "USPHS", false);
        self.add_table(table_0140);

        // HL7 Table 0141 - Military Rank/Grade
        let mut table_0141 = Hl7Table::new("0141", "Military Rank/Grade", "Military Rank/Grade");
        table_0141.add_value("E1", "E1", false);
        table_0141.add_value("E2", "E2", false);
        table_0141.add_value("E3", "E3", false);
        table_0141.add_value("E4", "E4", false);
        table_0141.add_value("E5", "E5", false);
        table_0141.add_value("E6", "E6", false);
        table_0141.add_value("E7", "E7", false);
        table_0141.add_value("E8", "E8", false);
        table_0141.add_value("E9", "E9", false);
        table_0141.add_value("O01", "O01", false);
        table_0141.add_value("O02", "O02", false);
        table_0141.add_value("O03", "O03", false);
        table_0141.add_value("O04", "O04", false);
        table_0141.add_value("O05", "O05", false);
        table_0141.add_value("O06", "O06", false);
        table_0141.add_value("O07", "O07", false);
        table_0141.add_value("O08", "O08", false);
        table_0141.add_value("O09", "O09", false);
        table_0141.add_value("O1", "O1", false);
        table_0141.add_value("O10", "O10", false);
        table_0141.add_value("O2", "O2", false);
        table_0141.add_value("O3", "O3", false);
        table_0141.add_value("O4", "O4", false);
        table_0141.add_value("O5", "O5", false);
        table_0141.add_value("O6", "O6", false);
        table_0141.add_value("O7", "O7", false);
        table_0141.add_value("O8", "O8", false);
        table_0141.add_value("O9", "O9", false);
        table_0141.add_value("W1", "W1", false);
        table_0141.add_value("W2", "W2", false);
        table_0141.add_value("W3", "W3", false);
        table_0141.add_value("W4", "W4", false);
        self.add_table(table_0141);

        // HL7 Table 0142 - Military Status
        let mut table_0142 = Hl7Table::new("0142", "Military Status", "Military Status");
        table_0142.add_value("ACT", "ACT", false);
        table_0142.add_value("DEC", "DEC", false);
        table_0142.add_value("RET", "RET", false);
        self.add_table(table_0142);

        // HL7 Table 0144 - Eligibility Source
        let mut table_0144 = Hl7Table::new("0144", "Eligibility Source", "Eligibility Source");
        table_0144.add_value("1", "1", false);
        table_0144.add_value("2", "2", false);
        table_0144.add_value("3", "3", false);
        table_0144.add_value("4", "4", false);
        table_0144.add_value("5", "5", false);
        table_0144.add_value("6", "6", false);
        table_0144.add_value("7", "7", false);
        self.add_table(table_0144);

        // HL7 Table 0145 - Room Type
        let mut table_0145 = Hl7Table::new("0145", "Room Type", "Room Type");
        table_0145.add_value("2ICU", "2ICU", false);
        table_0145.add_value("2PRI", "2PRI", false);
        table_0145.add_value("2SPR", "2SPR", false);
        table_0145.add_value("ICU", "ICU", false);
        table_0145.add_value("PRI", "PRI", false);
        table_0145.add_value("SPR", "SPR", false);
        self.add_table(table_0145);

        // HL7 Table 0146 - Amount Type
        let mut table_0146 = Hl7Table::new("0146", "Amount Type", "Amount Type");
        table_0146.add_value("DF", "DF", false);
        table_0146.add_value("LM", "LM", false);
        table_0146.add_value("PC", "PC", false);
        table_0146.add_value("RT", "RT", false);
        table_0146.add_value("UL", "UL", false);
        self.add_table(table_0146);

        // HL7 Table 0147 - Policy Type
        let mut table_0147 = Hl7Table::new("0147", "Policy Type", "Policy Type");
        table_0147.add_value("2ANC", "2ANC", false);
        table_0147.add_value("2MMD", "2MMD", false);
        table_0147.add_value("3MMD", "3MMD", false);
        table_0147.add_value("ANC", "ANC", false);
        table_0147.add_value("MMD", "MMD", false);
        table_0147.add_value("AT", "AT", false);
        table_0147.add_value("PC", "PC", false);
        self.add_table(table_0147);

        // HL7 Table 0148 - Money or Percentage Indicator
        let mut table_0148 = Hl7Table::new(
            "0148",
            "Money or Percentage Indicator",
            "Money or Percentage Indicator",
        );
        table_0148.add_value("AT", "AT", false);
        table_0148.add_value("PC", "PC", false);
        self.add_table(table_0148);

        // HL7 Table 0149 - Day Type
        let mut table_0149 = Hl7Table::new("0149", "Day Type", "Day Type");
        table_0149.add_value("AP", "AP", false);
        table_0149.add_value("DE", "DE", false);
        table_0149.add_value("PE", "PE", false);
        table_0149.add_value("ER", "ER", false);
        table_0149.add_value("IPE", "IPE", false);
        table_0149.add_value("OPE", "OPE", false);
        table_0149.add_value("UR", "UR", false);
        self.add_table(table_0149);

        // HL7 Table 0150 - Certification Patient Type
        let mut table_0150 = Hl7Table::new(
            "0150",
            "Certification Patient Type",
            "Certification Patient Type",
        );
        table_0150.add_value("ER", "ER", false);
        table_0150.add_value("IPE", "IPE", false);
        table_0150.add_value("OPE", "OPE", false);
        table_0150.add_value("UR", "UR", false);
        self.add_table(table_0150);

        // HL7 Table 0153 - Value Code
        let mut table_0153 = Hl7Table::new("0153", "Value Code", "Value Code");
        table_0153.add_value("01", "01", false);
        table_0153.add_value("02", "02", false);
        table_0153.add_value("04", "04", false);
        table_0153.add_value("05", "05", false);
        table_0153.add_value("06", "06", false);
        table_0153.add_value("08", "08", false);
        table_0153.add_value("09", "09", false);
        table_0153.add_value("10", "10", false);
        table_0153.add_value("11", "11", false);
        table_0153.add_value("12", "12", false);
        table_0153.add_value("13", "13", false);
        table_0153.add_value("14", "14", false);
        table_0153.add_value("15", "15", false);
        table_0153.add_value("16", "16", false);
        table_0153.add_value("17", "17", false);
        table_0153.add_value("21", "21", false);
        table_0153.add_value("22", "22", false);
        table_0153.add_value("23", "23", false);
        table_0153.add_value("24", "24", false);
        table_0153.add_value("30", "30", false);
        table_0153.add_value("31", "31", false);
        table_0153.add_value("37", "37", false);
        table_0153.add_value("38", "38", false);
        table_0153.add_value("39", "39", false);
        table_0153.add_value("40", "40", false);
        table_0153.add_value("41", "41", false);
        table_0153.add_value("42", "42", false);
        table_0153.add_value("43", "43", false);
        table_0153.add_value("44", "44", false);
        table_0153.add_value("45", "45", false);
        table_0153.add_value("46", "46", false);
        table_0153.add_value("47", "47", false);
        table_0153.add_value("48", "48", false);
        table_0153.add_value("49", "49", false);
        table_0153.add_value("50", "50", false);
        table_0153.add_value("51", "51", false);
        table_0153.add_value("52", "52", false);
        table_0153.add_value("53", "53", false);
        table_0153.add_value("56", "56", false);
        table_0153.add_value("57", "57", false);
        table_0153.add_value("58", "58", false);
        table_0153.add_value("59", "59", false);
        table_0153.add_value("60", "60", false);
        table_0153.add_value("67", "67", false);
        table_0153.add_value("68", "68", false);
        table_0153.add_value("70", "70", false);
        table_0153.add_value("71", "71", false);
        table_0153.add_value("72", "72", false);
        table_0153.add_value("75", "75", false);
        table_0153.add_value("76", "76", false);
        table_0153.add_value("77", "77", false);
        table_0153.add_value("78", "78", false);
        table_0153.add_value("79", "79", false);
        table_0153.add_value("80", "80", false);
        table_0153.add_value("81", "81", false);
        table_0153.add_value("A1", "A1", false);
        table_0153.add_value("A2", "A2", false);
        table_0153.add_value("A3", "A3", false);
        table_0153.add_value("X0", "X0", false);
        table_0153.add_value("X4", "X4", false);
        table_0153.add_value("â¦", "â¦", false);
        table_0153.add_value("AL", "AL", false);
        table_0153.add_value("ER", "ER", false);
        table_0153.add_value("NE", "NE", false);
        table_0153.add_value("SU", "SU", false);
        table_0153.add_value("ANY", "ANY", false);
        table_0153.add_value("COL", "COL", false);
        table_0153.add_value("ORD", "ORD", false);
        table_0153.add_value("RCT", "RCT", false);
        table_0153.add_value("REP", "REP", false);
        table_0153.add_value("SCHED", "SCHED", false);
        table_0153.add_value("CFN", "CFN", false);
        table_0153.add_value("COR", "COR", false);
        table_0153.add_value("FIN", "FIN", false);
        table_0153.add_value("PRE", "PRE", false);
        table_0153.add_value("1ST", "1ST", false);
        table_0153.add_value("ALL", "ALL", false);
        table_0153.add_value("LST", "LST", false);
        table_0153.add_value("REV", "REV", false);
        table_0153.add_value("D", "D", false);
        table_0153.add_value("P", "P", false);
        table_0153.add_value("S", "S", false);
        self.add_table(table_0153);

        // HL7 Table 0155 - Accept/Application Acknowledgment Conditions
        let mut table_0155 = Hl7Table::new(
            "0155",
            "Accept/Application Acknowledgment Conditions",
            "Accept/Application Acknowledgment Conditions",
        );
        table_0155.add_value("AL", "AL", false);
        table_0155.add_value("ER", "ER", false);
        table_0155.add_value("NE", "NE", false);
        table_0155.add_value("SU", "SU", false);
        self.add_table(table_0155);

        // HL7 Table 0156 - Which date/time qualifier
        let mut table_0156 = Hl7Table::new(
            "0156",
            "Which date/time qualifier",
            "Which date/time qualifier",
        );
        table_0156.add_value("ANY", "ANY", false);
        table_0156.add_value("COL", "COL", false);
        table_0156.add_value("ORD", "ORD", false);
        table_0156.add_value("RCT", "RCT", false);
        table_0156.add_value("REP", "REP", false);
        table_0156.add_value("SCHED", "SCHED", false);
        self.add_table(table_0156);

        // HL7 Table 0157 - Which date/time status qualifier
        let mut table_0157 = Hl7Table::new(
            "0157",
            "Which date/time status qualifier",
            "Which date/time status qualifier",
        );
        table_0157.add_value("ANY", "ANY", false);
        table_0157.add_value("CFN", "CFN", false);
        table_0157.add_value("COR", "COR", false);
        table_0157.add_value("FIN", "FIN", false);
        table_0157.add_value("PRE", "PRE", false);
        table_0157.add_value("REP", "REP", false);
        self.add_table(table_0157);

        // HL7 Table 0158 - Date/time selection qualifier
        let mut table_0158 = Hl7Table::new(
            "0158",
            "Date/time selection qualifier",
            "Date/time selection qualifier",
        );
        table_0158.add_value("1ST", "1ST", false);
        table_0158.add_value("ALL", "ALL", false);
        table_0158.add_value("LST", "LST", false);
        table_0158.add_value("REV", "REV", false);
        self.add_table(table_0158);

        // HL7 Table 0159 - Diet Code Specification Type
        let mut table_0159 = Hl7Table::new(
            "0159",
            "Diet Code Specification Type",
            "Diet Code Specification Type",
        );
        table_0159.add_value("D", "D", false);
        table_0159.add_value("P", "P", false);
        table_0159.add_value("S", "S", false);
        self.add_table(table_0159);

        // HL7 Table 0160 - Tray Type
        let mut table_0160 = Hl7Table::new("0160", "Tray Type", "Tray Type");
        table_0160.add_value("EARLY", "EARLY", false);
        table_0160.add_value("GUEST", "GUEST", false);
        table_0160.add_value("LATE", "LATE", false);
        table_0160.add_value("MSG", "MSG", false);
        table_0160.add_value("NO", "NO", false);
        self.add_table(table_0160);

        // HL7 Table 0161 - Allow Substitution
        let mut table_0161 = Hl7Table::new("0161", "Allow Substitution", "Allow Substitution");
        table_0161.add_value("G", "G", false);
        table_0161.add_value("N", "N", false);
        table_0161.add_value("T", "T", false);
        self.add_table(table_0161);

        // HL7 Table 0162 - Route of Administration
        let mut table_0162 =
            Hl7Table::new("0162", "Route of Administration", "Route of Administration");
        table_0162.add_value("AP", "AP", false);
        table_0162.add_value("B", "B", false);
        table_0162.add_value("DT", "DT", false);
        table_0162.add_value("EP", "EP", false);
        table_0162.add_value("ET", "ET", false);
        table_0162.add_value("GTT", "GTT", false);
        table_0162.add_value("GU", "GU", false);
        table_0162.add_value("IA", "IA", false);
        table_0162.add_value("IB", "IB", false);
        table_0162.add_value("IC", "IC", false);
        table_0162.add_value("ICV", "ICV", false);
        table_0162.add_value("ID", "ID", false);
        table_0162.add_value("IH", "IH", false);
        table_0162.add_value("IHA", "IHA", false);
        table_0162.add_value("IM", "IM", false);
        table_0162.add_value("IMR", "IMR", false);
        table_0162.add_value("IN", "IN", false);
        table_0162.add_value("IO", "IO", false);
        table_0162.add_value("IP", "IP", false);
        table_0162.add_value("IS", "IS", false);
        table_0162.add_value("IT", "IT", false);
        table_0162.add_value("IU", "IU", false);
        table_0162.add_value("IV", "IV", false);
        table_0162.add_value("MM", "MM", false);
        table_0162.add_value("MTH", "MTH", false);
        table_0162.add_value("NG", "NG", false);
        table_0162.add_value("NP", "NP", false);
        table_0162.add_value("NS", "NS", false);
        table_0162.add_value("NT", "NT", false);
        table_0162.add_value("OP", "OP", false);
        table_0162.add_value("OT", "OT", false);
        table_0162.add_value("OTH", "OTH", false);
        table_0162.add_value("PF", "PF", false);
        table_0162.add_value("PO", "PO", false);
        table_0162.add_value("PR", "PR", false);
        table_0162.add_value("RM", "RM", false);
        table_0162.add_value("SC", "SC", false);
        table_0162.add_value("SD", "SD", false);
        table_0162.add_value("SL", "SL", false);
        table_0162.add_value("TD", "TD", false);
        table_0162.add_value("TL", "TL", false);
        table_0162.add_value("TP", "TP", false);
        table_0162.add_value("TRA", "TRA", false);
        table_0162.add_value("UR", "UR", false);
        table_0162.add_value("VG", "VG", false);
        table_0162.add_value("VM", "VM", false);
        table_0162.add_value("WND", "WND", false);
        self.add_table(table_0162);

        // HL7 Table 0163 - Body Site
        let mut table_0163 = Hl7Table::new("0163", "Body Site", "Body Site");
        table_0163.add_value("BE", "BE", false);
        table_0163.add_value("BN", "BN", false);
        table_0163.add_value("BU", "BU", false);
        table_0163.add_value("CT", "CT", false);
        table_0163.add_value("LA", "LA", false);
        table_0163.add_value("LAC", "LAC", false);
        table_0163.add_value("LACF", "LACF", false);
        table_0163.add_value("LD", "LD", false);
        table_0163.add_value("LE", "LE", false);
        table_0163.add_value("LEJ", "LEJ", false);
        table_0163.add_value("LF", "LF", false);
        table_0163.add_value("LG", "LG", false);
        table_0163.add_value("LH", "LH", false);
        table_0163.add_value("LIJ", "LIJ", false);
        table_0163.add_value("LLAQ", "LLAQ", false);
        table_0163.add_value("LLFA", "LLFA", false);
        table_0163.add_value("LMFA", "LMFA", false);
        table_0163.add_value("LN", "LN", false);
        table_0163.add_value("LPC", "LPC", false);
        table_0163.add_value("LSC", "LSC", false);
        table_0163.add_value("LT", "LT", false);
        table_0163.add_value("LUA", "LUA", false);
        table_0163.add_value("LUAQ", "LUAQ", false);
        table_0163.add_value("LUFA", "LUFA", false);
        table_0163.add_value("LVG", "LVG", false);
        table_0163.add_value("LVL", "LVL", false);
        table_0163.add_value("NB", "NB", false);
        table_0163.add_value("OD", "OD", false);
        table_0163.add_value("OS", "OS", false);
        table_0163.add_value("OU", "OU", false);
        table_0163.add_value("PA", "PA", false);
        table_0163.add_value("PERIN", "PERIN", false);
        table_0163.add_value("RA", "RA", false);
        table_0163.add_value("RAC", "RAC", false);
        table_0163.add_value("RACF", "RACF", false);
        table_0163.add_value("RD", "RD", false);
        table_0163.add_value("RE", "RE", false);
        table_0163.add_value("REJ", "REJ", false);
        table_0163.add_value("RF", "RF", false);
        table_0163.add_value("RG", "RG", false);
        table_0163.add_value("RH", "RH", false);
        table_0163.add_value("RIJ", "RIJ", false);
        table_0163.add_value("RLAQ", "RLAQ", false);
        table_0163.add_value("RLFA", "RLFA", false);
        table_0163.add_value("RMFA", "RMFA", false);
        table_0163.add_value("RN", "RN", false);
        table_0163.add_value("RPC", "RPC", false);
        table_0163.add_value("RSC", "RSC", false);
        table_0163.add_value("RT", "RT", false);
        table_0163.add_value("RUA", "RUA", false);
        table_0163.add_value("RUAQ", "RUAQ", false);
        table_0163.add_value("RUFA", "RUFA", false);
        table_0163.add_value("RVG", "RVG", false);
        table_0163.add_value("RVL", "RVL", false);
        self.add_table(table_0163);

        // HL7 Table 0164 - Administration Device
        let mut table_0164 =
            Hl7Table::new("0164", "Administration Device", "Administration Device");
        table_0164.add_value("AP", "AP", false);
        table_0164.add_value("BT", "BT", false);
        table_0164.add_value("HL", "HL", false);
        table_0164.add_value("IPPB", "IPPB", false);
        table_0164.add_value("IVP", "IVP", false);
        table_0164.add_value("IVS", "IVS", false);
        table_0164.add_value("MI", "MI", false);
        table_0164.add_value("NEB", "NEB", false);
        table_0164.add_value("PCA", "PCA", false);
        self.add_table(table_0164);

        // HL7 Table 0165 - Administration Method
        let mut table_0165 =
            Hl7Table::new("0165", "Administration Method", "Administration Method");
        table_0165.add_value("CH", "CH", false);
        table_0165.add_value("DI", "DI", false);
        table_0165.add_value("DU", "DU", false);
        table_0165.add_value("IF", "IF", false);
        table_0165.add_value("IR", "IR", false);
        table_0165.add_value("IS", "IS", false);
        table_0165.add_value("IVP", "IVP", false);
        table_0165.add_value("IVPB", "IVPB", false);
        table_0165.add_value("NB", "NB", false);
        table_0165.add_value("PF", "PF", false);
        table_0165.add_value("PT", "PT", false);
        table_0165.add_value("SH", "SH", false);
        table_0165.add_value("SO", "SO", false);
        table_0165.add_value("WA", "WA", false);
        table_0165.add_value("WI", "WI", false);
        self.add_table(table_0165);

        // HL7 Table 0166 - RX Component Type
        let mut table_0166 = Hl7Table::new("0166", "RX Component Type", "RX Component Type");
        table_0166.add_value("A", "A", false);
        table_0166.add_value("B", "B", false);
        self.add_table(table_0166);

        // HL7 Table 0167 - Substitution Status
        let mut table_0167 = Hl7Table::new("0167", "Substitution Status", "Substitution Status");
        table_0167.add_value("0", "0", false);
        table_0167.add_value("1", "1", false);
        table_0167.add_value("2", "2", false);
        table_0167.add_value("3", "3", false);
        table_0167.add_value("4", "4", false);
        table_0167.add_value("5", "5", false);
        table_0167.add_value("7", "7", false);
        table_0167.add_value("8", "8", false);
        table_0167.add_value("G", "G", false);
        table_0167.add_value("N", "N", false);
        table_0167.add_value("T", "T", false);
        table_0167.add_value("CO", "CO", false);
        table_0167.add_value("IN", "IN", false);
        table_0167.add_value("CDM", "CDM", false);
        table_0167.add_value("CLN", "CLN", false);
        table_0167.add_value("CMA", "CMA", false);
        table_0167.add_value("CMB", "CMB", false);
        table_0167.add_value("INV", "INV", false);
        table_0167.add_value("LOC", "LOC", false);
        table_0167.add_value("MACP", "MACP", false);
        table_0167.add_value("MLCP", "MLCP", false);
        table_0167.add_value("OMA", "OMA", false);
        table_0167.add_value("OMB", "OMB", false);
        table_0167.add_value("OMC", "OMC", false);
        table_0167.add_value("OMD", "OMD", false);
        table_0167.add_value("OME", "OME", false);
        table_0167.add_value("OMM", "OMM", false);
        table_0167.add_value("PRA", "PRA", false);
        table_0167.add_value("STF", "STF", false);
        self.add_table(table_0167);

        // HL7 Table 0173 - Coordination of Benefits
        let mut table_0173 = Hl7Table::new(
            "0173",
            "Coordination of Benefits",
            "Coordination of Benefits",
        );
        table_0173.add_value("CO", "CO", false);
        table_0173.add_value("IN", "IN", false);
        self.add_table(table_0173);

        // HL7 Table 0175 - Master File Identifier Code
        let mut table_0175 = Hl7Table::new(
            "0175",
            "Master File Identifier Code",
            "Master File Identifier Code",
        );
        table_0175.add_value("CDM", "CDM", false);
        table_0175.add_value("CLN", "CLN", false);
        table_0175.add_value("CMA", "CMA", false);
        table_0175.add_value("CMB", "CMB", false);
        table_0175.add_value("INV", "INV", false);
        table_0175.add_value("LOC", "LOC", false);
        table_0175.add_value("MACP", "MACP", false);
        table_0175.add_value("MLCP", "MLCP", false);
        table_0175.add_value("OMA", "OMA", false);
        table_0175.add_value("OMB", "OMB", false);
        table_0175.add_value("OMC", "OMC", false);
        table_0175.add_value("OMD", "OMD", false);
        table_0175.add_value("OME", "OME", false);
        table_0175.add_value("OMM", "OMM", false);
        table_0175.add_value("PRA", "PRA", false);
        table_0175.add_value("STF", "STF", false);
        self.add_table(table_0175);

        // HL7 Table 0177 - Confidentiality Code
        let mut table_0177 = Hl7Table::new("0177", "Confidentiality Code", "Confidentiality Code");
        table_0177.add_value("AID", "AID", false);
        table_0177.add_value("EMP", "EMP", false);
        table_0177.add_value("ETH", "ETH", false);
        table_0177.add_value("HIV", "HIV", false);
        table_0177.add_value("PSY", "PSY", false);
        table_0177.add_value("R", "R", false);
        table_0177.add_value("U", "U", false);
        table_0177.add_value("UWM", "UWM", false);
        table_0177.add_value("V", "V", false);
        table_0177.add_value("VIP", "VIP", false);
        self.add_table(table_0177);

        // HL7 Table 0178 - File Level Event Code
        let mut table_0178 =
            Hl7Table::new("0178", "File Level Event Code", "File Level Event Code");
        table_0178.add_value("REP", "REP", false);
        table_0178.add_value("UPD", "UPD", false);
        self.add_table(table_0178);

        // HL7 Table 0179 - Response Level
        let mut table_0179 = Hl7Table::new("0179", "Response Level", "Response Level");
        table_0179.add_value("AL", "AL", false);
        table_0179.add_value("ER", "ER", false);
        table_0179.add_value("NE", "NE", false);
        table_0179.add_value("SU", "SU", false);
        self.add_table(table_0179);

        // HL7 Table 0180 - Record-level Event Code
        let mut table_0180 =
            Hl7Table::new("0180", "Record-level Event Code", "Record-level Event Code");
        table_0180.add_value("MAC", "MAC", false);
        table_0180.add_value("MAD", "MAD", false);
        table_0180.add_value("MDC", "MDC", false);
        table_0180.add_value("MDL", "MDL", false);
        table_0180.add_value("MUP", "MUP", false);
        self.add_table(table_0180);

        // HL7 Table 0185 - Preferred Method of Contact
        let mut table_0185 = Hl7Table::new(
            "0185",
            "Preferred Method of Contact",
            "Preferred Method of Contact",
        );
        table_0185.add_value("B", "B", false);
        table_0185.add_value("C", "C", false);
        table_0185.add_value("E", "E", false);
        table_0185.add_value("F", "F", false);
        table_0185.add_value("H", "H", false);
        table_0185.add_value("O", "O", false);
        self.add_table(table_0185);

        // HL7 Table 0189 - Ethnic Group
        let mut table_0189 = Hl7Table::new("0189", "Ethnic Group", "Ethnic Group");
        table_0189.add_value("H", "H", false);
        table_0189.add_value("N", "N", false);
        table_0189.add_value("U", "U", false);
        self.add_table(table_0189);

        // HL7 Table 0190 - Address Type
        let mut table_0190 = Hl7Table::new("0190", "Address Type", "Address Type");
        table_0190.add_value("B", "B", false);
        table_0190.add_value("BA", "BA", false);
        table_0190.add_value("BDL", "BDL", false);
        table_0190.add_value("BI", "BI", false);
        table_0190.add_value("BR", "BR", false);
        table_0190.add_value("C", "C", false);
        table_0190.add_value("F", "F", false);
        table_0190.add_value("H", "H", false);
        table_0190.add_value("L", "L", false);
        table_0190.add_value("M", "M", false);
        table_0190.add_value("N", "N", false);
        table_0190.add_value("O", "O", false);
        table_0190.add_value("P", "P", false);
        table_0190.add_value("RH", "RH", false);
        table_0190.add_value("S", "S", false);
        table_0190.add_value("SH", "SH", false);
        table_0190.add_value("TM", "TM", false);
        table_0190.add_value("V", "V", false);
        self.add_table(table_0190);

        // HL7 Table 0191 - Type of Referenced Data
        let mut table_0191 =
            Hl7Table::new("0191", "Type of Referenced Data", "Type of Referenced Data");
        table_0191.add_value("AP", "AP", false);
        table_0191.add_value("AU", "AU", false);
        table_0191.add_value("Application", "Application", false);
        table_0191.add_value("Audio", "Audio", false);
        table_0191.add_value("FT", "FT", false);
        table_0191.add_value("IM", "IM", false);
        table_0191.add_value("Image", "Image", false);
        table_0191.add_value("NS", "NS", false);
        table_0191.add_value("SD", "SD", false);
        table_0191.add_value("SGML", "SGML", false);
        table_0191.add_value("SI", "SI", false);
        table_0191.add_value("TEXT", "TEXT", false);
        table_0191.add_value("TX", "TX", false);
        table_0191.add_value("multipart", "multipart", false);
        self.add_table(table_0191);

        // HL7 Table 0193 - Amount Class
        let mut table_0193 = Hl7Table::new("0193", "Amount Class", "Amount Class");
        table_0193.add_value("AT", "AT", false);
        table_0193.add_value("LM", "LM", false);
        table_0193.add_value("PC", "PC", false);
        table_0193.add_value("UL", "UL", false);
        self.add_table(table_0193);

        // HL7 Table 0200 - Name Type
        let mut table_0200 = Hl7Table::new("0200", "Name Type", "Name Type");
        table_0200.add_value("A", "A", false);
        table_0200.add_value("B", "B", false);
        table_0200.add_value("BAD", "BAD", false);
        table_0200.add_value("C", "C", false);
        table_0200.add_value("D", "D", false);
        table_0200.add_value("F", "F", false);
        table_0200.add_value("I", "I", false);
        table_0200.add_value("K", "K", false);
        table_0200.add_value("L", "L", false);
        table_0200.add_value("M", "M", false);
        table_0200.add_value("MSK", "MSK", false);
        table_0200.add_value("N", "N", false);
        table_0200.add_value("NAV", "NAV", false);
        table_0200.add_value("NB", "NB", false);
        table_0200.add_value("NOUSE", "NOUSE", false);
        table_0200.add_value("P", "P", false);
        table_0200.add_value("R", "R", false);
        table_0200.add_value("REL", "REL", false);
        table_0200.add_value("S", "S", false);
        table_0200.add_value("T", "T", false);
        table_0200.add_value("TEMP", "TEMP", false);
        table_0200.add_value("U", "U", false);
        self.add_table(table_0200);

        // HL7 Table 0201 - Telecommunication Use Code
        let mut table_0201 =
            Hl7Table::new("0201", "Telecommunication Use Code", "Purpose of telecom");
        table_0201.add_value("PRN", "Primary residence number", false);
        table_0201.add_value("ORN", "Other residence number", false);
        table_0201.add_value("WPN", "Work number", false);
        table_0201.add_value("VHN", "Vacation home number", false);
        table_0201.add_value("ASN", "Answering service number", false);
        table_0201.add_value("EMR", "Emergency number", false);
        table_0201.add_value("NET", "Network (email) address", false);
        table_0201.add_value("BPN", "Beeper number", false);
        table_0201.add_value("PRS", "PRS", false);
        table_0201.add_value("BP", "BP", false);
        table_0201.add_value("CP", "CP", false);
        table_0201.add_value("FX", "FX", false);
        table_0201.add_value("Internet", "Internet", false);
        table_0201.add_value("MD", "MD", false);
        table_0201.add_value("PH", "PH", false);
        table_0201.add_value("SAT", "SAT", false);
        table_0201.add_value("TDD", "TDD", false);
        table_0201.add_value("TTY", "TTY", false);
        table_0201.add_value("X.400", "X.400", false);
        self.add_table(table_0201);

        // HL7 Table 0202 - Telecommunication Equipment Type
        let mut table_0202 = Hl7Table::new(
            "0202",
            "Telecommunication Equipment Type",
            "Telecommunication Equipment Type",
        );
        table_0202.add_value("BP", "BP", false);
        table_0202.add_value("CP", "CP", false);
        table_0202.add_value("FX", "FX", false);
        table_0202.add_value("Internet", "Internet", false);
        table_0202.add_value("MD", "MD", false);
        table_0202.add_value("PH", "PH", false);
        table_0202.add_value("SAT", "SAT", false);
        table_0202.add_value("TDD", "TDD", false);
        table_0202.add_value("TTY", "TTY", false);
        table_0202.add_value("X.400", "X.400", false);
        self.add_table(table_0202);

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
        table_0203.add_value("ACSN", "ACSN", false);
        table_0203.add_value("AMA", "AMA", false);
        table_0203.add_value("ANC", "ANC", false);
        table_0203.add_value("AND", "AND", false);
        table_0203.add_value("ANON", "ANON", false);
        table_0203.add_value("ANT", "ANT", false);
        table_0203.add_value("APRN", "APRN", false);
        table_0203.add_value("ASID", "ASID", false);
        table_0203.add_value("BA", "BA", false);
        table_0203.add_value("BC", "BC", false);
        table_0203.add_value("BCT", "BCT", false);
        table_0203.add_value("BRN", "BRN", false);
        table_0203.add_value("BSNR", "BSNR", false);
        table_0203.add_value("CC", "CC", false);
        table_0203.add_value("CONM", "CONM", false);
        table_0203.add_value("CY", "CY", false);
        table_0203.add_value("CZ", "CZ", false);
        table_0203.add_value("DDS", "DDS", false);
        table_0203.add_value("DEA", "DEA", false);
        table_0203.add_value("DFN", "DFN", false);
        table_0203.add_value("DO", "DO", false);
        table_0203.add_value("DP", "DP", false);
        table_0203.add_value("DPM", "DPM", false);
        table_0203.add_value("DR", "DR", false);
        table_0203.add_value("ESN", "ESN", false);
        table_0203.add_value("FI", "FI", false);
        table_0203.add_value("GI", "GI", false);
        table_0203.add_value("GL", "GL", false);
        table_0203.add_value("GN", "GN", false);
        table_0203.add_value("HC", "HC", false);
        table_0203.add_value("IND", "IND", false);
        table_0203.add_value("JHN", "JHN", false);
        table_0203.add_value("LACSN", "LACSN", false);
        table_0203.add_value("LANR", "LANR", false);
        table_0203.add_value("LI", "LI", false);
        table_0203.add_value("LN", "LN", false);
        table_0203.add_value("LR", "LR", false);
        table_0203.add_value("MB", "MB", false);
        table_0203.add_value("MCD", "MCD", false);
        table_0203.add_value("MCN", "MCN", false);
        table_0203.add_value("MCR", "MCR", false);
        table_0203.add_value("MCT", "MCT", false);
        table_0203.add_value("MD", "MD", false);
        table_0203.add_value("MI", "MI", false);
        table_0203.add_value("MRT", "MRT", false);
        table_0203.add_value("NBSNR", "NBSNR", false);
        table_0203.add_value("NCT", "NCT", false);
        table_0203.add_value("NII", "NII", false);
        table_0203.add_value("NIIP", "NIIP", false);
        table_0203.add_value("NP", "NP", false);
        table_0203.add_value("OD", "OD", false);
        table_0203.add_value("PA", "PA", false);
        table_0203.add_value("PC", "PC", false);
        table_0203.add_value("PCN", "PCN", false);
        table_0203.add_value("PE", "PE", false);
        table_0203.add_value("PNT", "PNT", false);
        table_0203.add_value("PPIN", "PPIN", false);
        table_0203.add_value("PPN", "PPN", false);
        table_0203.add_value("PRC", "PRC", false);
        table_0203.add_value("PRN", "PRN", false);
        table_0203.add_value("PT", "PT", false);
        table_0203.add_value("QA", "QA", false);
        table_0203.add_value("RI", "RI", false);
        table_0203.add_value("RN", "RN", false);
        table_0203.add_value("RPH", "RPH", false);
        table_0203.add_value("RRI", "RRI", false);
        table_0203.add_value("RRP", "RRP", false);
        table_0203.add_value("SID", "SID", false);
        table_0203.add_value("SL", "SL", false);
        table_0203.add_value("SP", "SP", false);
        table_0203.add_value("TAX", "TAX", false);
        table_0203.add_value("TN", "TN", false);
        table_0203.add_value("TPR", "TPR", false);
        table_0203.add_value("UPIN", "UPIN", false);
        table_0203.add_value("USID", "USID", false);
        table_0203.add_value("VP", "VP", false);
        table_0203.add_value("WC", "WC", false);
        table_0203.add_value("WCN", "WCN", false);
        table_0203.add_value("WP", "WP", false);
        table_0203.add_value("A", "A", false);
        table_0203.add_value("D", "D", false);
        table_0203.add_value("L", "L", false);
        table_0203.add_pattern("NNxxx");
        self.add_table(table_0203);

        // HL7 Table 0204 - Organizational Name Type
        let mut table_0204 = Hl7Table::new(
            "0204",
            "Organizational Name Type",
            "Organizational Name Type",
        );
        table_0204.add_value("A", "A", false);
        table_0204.add_value("D", "D", false);
        table_0204.add_value("L", "L", false);
        table_0204.add_value("SL", "SL", false);
        self.add_table(table_0204);

        // HL7 Table 0205 - Price Type
        let mut table_0205 = Hl7Table::new("0205", "Price Type", "Price Type");
        table_0205.add_value("AP", "AP", false);
        table_0205.add_value("DC", "DC", false);
        table_0205.add_value("IC", "IC", false);
        table_0205.add_value("PF", "PF", false);
        table_0205.add_value("TF", "TF", false);
        table_0205.add_value("TP", "TP", false);
        table_0205.add_value("UP", "UP", false);
        self.add_table(table_0205);

        // HL7 Table 0206 - Segment Action Code
        let mut table_0206 = Hl7Table::new("0206", "Segment Action Code", "Segment Action Code");
        table_0206.add_value("A", "A", false);
        table_0206.add_value("D", "D", false);
        table_0206.add_value("U", "U", false);
        table_0206.add_value("X", "X", false);
        self.add_table(table_0206);

        // HL7 Table 0207 - Processing Mode
        let mut table_0207 = Hl7Table::new("0207", "Processing Mode", "Processing Mode");
        table_0207.add_value("A", "A", false);
        table_0207.add_value("I", "I", false);
        table_0207.add_value("R", "R", false);
        table_0207.add_value("T", "T", false);
        table_0207.add_value("8859/1", "8859/1", false);
        table_0207.add_value("8859/15", "8859/15", false);
        table_0207.add_value("8859/2", "8859/2", false);
        table_0207.add_value("8859/3", "8859/3", false);
        table_0207.add_value("8859/4", "8859/4", false);
        table_0207.add_value("8859/5", "8859/5", false);
        table_0207.add_value("8859/6", "8859/6", false);
        table_0207.add_value("8859/7", "8859/7", false);
        table_0207.add_value("8859/8", "8859/8", false);
        table_0207.add_value("8859/9", "8859/9", false);
        table_0207.add_value("ASCII", "ASCII", false);
        table_0207.add_value("BIG-5", "BIG-5", false);
        table_0207.add_value("CNS 11643-1992", "CNS 11643-1992", false);
        table_0207.add_value("GB 18030-2000", "GB 18030-2000", false);
        table_0207.add_value("ISO IR14", "ISO IR14", false);
        table_0207.add_value("ISO IR159", "ISO IR159", false);
        table_0207.add_value("ISO IR6", "ISO IR6", false);
        table_0207.add_value("ISO IR87", "ISO IR87", false);
        table_0207.add_value("KS X 1001", "KS X 1001", false);
        table_0207.add_value("UNICODE", "UNICODE", false);
        table_0207.add_value("UNICODE UTF-16", "UNICODE UTF-16", false);
        table_0207.add_value("UNICODE UTF-32", "UNICODE UTF-32", false);
        table_0207.add_value("UNICODE UTF-8", "UNICODE UTF-8", false);
        self.add_table(table_0207);

        // HL7 Table 0211 - Alternate Character Sets
        let mut table_0211 = Hl7Table::new(
            "0211",
            "Alternate Character Sets",
            "Alternate Character Sets",
        );
        table_0211.add_value("8859/1", "8859/1", false);
        table_0211.add_value("8859/15", "8859/15", false);
        table_0211.add_value("8859/2", "8859/2", false);
        table_0211.add_value("8859/3", "8859/3", false);
        table_0211.add_value("8859/4", "8859/4", false);
        table_0211.add_value("8859/5", "8859/5", false);
        table_0211.add_value("8859/6", "8859/6", false);
        table_0211.add_value("8859/7", "8859/7", false);
        table_0211.add_value("8859/8", "8859/8", false);
        table_0211.add_value("8859/9", "8859/9", false);
        table_0211.add_value("ASCII", "ASCII", false);
        table_0211.add_value("BIG-5", "BIG-5", false);
        table_0211.add_value("CNS 11643-1992", "CNS 11643-1992", false);
        table_0211.add_value("GB 18030-2000", "GB 18030-2000", false);
        table_0211.add_value("ISO IR14", "ISO IR14", false);
        table_0211.add_value("ISO IR159", "ISO IR159", false);
        table_0211.add_value("ISO IR6", "ISO IR6", false);
        table_0211.add_value("ISO IR87", "ISO IR87", false);
        table_0211.add_value("KS X 1001", "KS X 1001", false);
        table_0211.add_value("UNICODE", "UNICODE", false);
        table_0211.add_value("UNICODE UTF-16", "UNICODE UTF-16", false);
        table_0211.add_value("UNICODE UTF-32", "UNICODE UTF-32", false);
        table_0211.add_value("UNICODE UTF-8", "UNICODE UTF-8", false);
        self.add_table(table_0211);

        // HL7 Table 0213 - Purge Status Code
        let mut table_0213 = Hl7Table::new("0213", "Purge Status Code", "Purge Status Code");
        table_0213.add_value("D", "D", false);
        table_0213.add_value("I", "I", false);
        table_0213.add_value("P", "P", false);
        self.add_table(table_0213);

        // HL7 Table 0214 - Special Program Code
        let mut table_0214 = Hl7Table::new("0214", "Special Program Code", "Special Program Code");
        table_0214.add_value("CH", "CH", false);
        table_0214.add_value("ES", "ES", false);
        table_0214.add_value("FP", "FP", false);
        table_0214.add_value("O", "O", false);
        table_0214.add_value("U", "U", false);
        self.add_table(table_0214);

        // HL7 Table 0215 - Publicity Code
        let mut table_0215 = Hl7Table::new("0215", "Publicity Code", "Publicity Code");
        table_0215.add_value("F", "F", false);
        table_0215.add_value("N", "N", false);
        table_0215.add_value("O", "O", false);
        table_0215.add_value("U", "U", false);
        self.add_table(table_0215);

        // HL7 Table 0216 - Patient Status Code
        let mut table_0216 = Hl7Table::new("0216", "Patient Status Code", "Patient Status Code");
        table_0216.add_value("AI", "AI", false);
        table_0216.add_value("DI", "DI", false);
        self.add_table(table_0216);

        // HL7 Table 0217 - Visit Priority Code
        let mut table_0217 = Hl7Table::new("0217", "Visit Priority Code", "Visit Priority Code");
        table_0217.add_value("1", "1", false);
        table_0217.add_value("2", "2", false);
        table_0217.add_value("3", "3", false);
        self.add_table(table_0217);

        // HL7 Table 0220 - Living Arrangement
        let mut table_0220 = Hl7Table::new("0220", "Living Arrangement", "Living Arrangement");
        table_0220.add_value("A", "A", false);
        table_0220.add_value("F", "F", false);
        table_0220.add_value("I", "I", false);
        table_0220.add_value("R", "R", false);
        table_0220.add_value("S", "S", false);
        table_0220.add_value("U", "U", false);
        self.add_table(table_0220);

        // HL7 Table 0223 - Living Dependency
        let mut table_0223 = Hl7Table::new("0223", "Living Dependency", "Living Dependency");
        table_0223.add_value("C", "C", false);
        table_0223.add_value("CB", "CB", false);
        table_0223.add_value("D", "D", false);
        table_0223.add_value("M", "M", false);
        table_0223.add_value("O", "O", false);
        table_0223.add_value("S", "S", false);
        table_0223.add_value("U", "U", false);
        table_0223.add_value("WU", "WU", false);
        self.add_table(table_0223);

        // HL7 Table 0224 - Transport Arranged
        let mut table_0224 = Hl7Table::new("0224", "Transport Arranged", "Transport Arranged");
        table_0224.add_value("A", "A", false);
        table_0224.add_value("N", "N", false);
        table_0224.add_value("U", "U", false);
        self.add_table(table_0224);

        // HL7 Table 0225 - Escort Required
        let mut table_0225 = Hl7Table::new("0225", "Escort Required", "Escort Required");
        table_0225.add_value("N", "N", false);
        table_0225.add_value("R", "R", false);
        table_0225.add_value("U", "U", false);
        table_0225.add_value("AB", "AB", false);
        table_0225.add_value("AD", "AD", false);
        table_0225.add_value("ALP", "ALP", false);
        table_0225.add_value("AR", "AR", false);
        table_0225.add_value("AVB", "AVB", false);
        table_0225.add_value("AVI", "AVI", false);
        table_0225.add_value("BA", "BA", false);
        table_0225.add_value("BAH", "BAH", false);
        table_0225.add_value("BAY", "BAY", false);
        table_0225.add_value("BP", "BP", false);
        table_0225.add_value("BPC", "BPC", false);
        table_0225.add_value("CEN", "CEN", false);
        table_0225.add_value("CHI", "CHI", false);
        table_0225.add_value("CMP", "CMP", false);
        table_0225.add_value("CNJ", "CNJ", false);
        table_0225.add_value("CON", "CON", false);
        table_0225.add_value("DVC", "DVC", false);
        table_0225.add_value("EVN", "EVN", false);
        table_0225.add_value("GEO", "GEO", false);
        table_0225.add_value("GRE", "GRE", false);
        table_0225.add_value("IAG", "IAG", false);
        table_0225.add_value("IM", "IM", false);
        table_0225.add_value("IUS", "IUS", false);
        table_0225.add_value("JPN", "JPN", false);
        table_0225.add_value("KGC", "KGC", false);
        table_0225.add_value("LED", "LED", false);
        table_0225.add_value("MA", "MA", false);
        table_0225.add_value("MBL", "MBL", false);
        table_0225.add_value("MED", "MED", false);
        table_0225.add_value("MIL", "MIL", false);
        table_0225.add_value("MIP", "MIP", false);
        table_0225.add_value("MSD", "MSD", false);
        table_0225.add_value("NAB", "NAB", false);
        table_0225.add_value("NAV", "NAV", false);
        table_0225.add_value("NOV", "NOV", false);
        table_0225.add_value("NVX", "NVX", false);
        table_0225.add_value("NYB", "NYB", false);
        table_0225.add_value("ORT", "ORT", false);
        table_0225.add_value("OTC", "OTC", false);
        table_0225.add_value("OTH", "OTH", false);
        table_0225.add_value("PD", "PD", false);
        table_0225.add_value("PMC", "PMC", false);
        table_0225.add_value("PRX", "PRX", false);
        table_0225.add_value("PWJ", "PWJ", false);
        table_0225.add_value("SCL", "SCL", false);
        table_0225.add_value("SI", "SI", false);
        table_0225.add_value("SKB", "SKB", false);
        table_0225.add_value("SOL", "SOL", false);
        table_0225.add_value("TAL", "TAL", false);
        table_0225.add_value("UNK", "UNK", false);
        table_0225.add_value("USA", "USA", false);
        table_0225.add_value("VXG", "VXG", false);
        table_0225.add_value("WA", "WA", false);
        table_0225.add_value("WAL", "WAL", false);
        table_0225.add_value("ZLB", "ZLB", false);
        table_0225.add_value("C", "C", false);
        table_0225.add_value("D", "D", false);
        table_0225.add_value("I", "I", false);
        table_0225.add_value("M", "M", false);
        table_0225.add_value("O", "O", false);
        table_0225.add_value("S", "S", false);
        table_0225.add_value("T", "T", false);
        self.add_table(table_0225);

        // HL7 Table 0227 - Manufacturers of Vaccines (code=MVX)
        let mut table_0227 = Hl7Table::new(
            "0227",
            "Manufacturers of Vaccines (code=MVX)",
            "Manufacturers of Vaccines (code=MVX)",
        );
        table_0227.add_value("AB", "AB", false);
        table_0227.add_value("AD", "AD", false);
        table_0227.add_value("ALP", "ALP", false);
        table_0227.add_value("AR", "AR", false);
        table_0227.add_value("AVB", "AVB", false);
        table_0227.add_value("AVI", "AVI", false);
        table_0227.add_value("BA", "BA", false);
        table_0227.add_value("BAH", "BAH", false);
        table_0227.add_value("BAY", "BAY", false);
        table_0227.add_value("BP", "BP", false);
        table_0227.add_value("BPC", "BPC", false);
        table_0227.add_value("CEN", "CEN", false);
        table_0227.add_value("CHI", "CHI", false);
        table_0227.add_value("CMP", "CMP", false);
        table_0227.add_value("CNJ", "CNJ", false);
        table_0227.add_value("CON", "CON", false);
        table_0227.add_value("DVC", "DVC", false);
        table_0227.add_value("EVN", "EVN", false);
        table_0227.add_value("GEO", "GEO", false);
        table_0227.add_value("GRE", "GRE", false);
        table_0227.add_value("IAG", "IAG", false);
        table_0227.add_value("IM", "IM", false);
        table_0227.add_value("IUS", "IUS", false);
        table_0227.add_value("JPN", "JPN", false);
        table_0227.add_value("KGC", "KGC", false);
        table_0227.add_value("LED", "LED", false);
        table_0227.add_value("MA", "MA", false);
        table_0227.add_value("MBL", "MBL", false);
        table_0227.add_value("MED", "MED", false);
        table_0227.add_value("MIL", "MIL", false);
        table_0227.add_value("MIP", "MIP", false);
        table_0227.add_value("MSD", "MSD", false);
        table_0227.add_value("NAB", "NAB", false);
        table_0227.add_value("NAV", "NAV", false);
        table_0227.add_value("NOV", "NOV", false);
        table_0227.add_value("NVX", "NVX", false);
        table_0227.add_value("NYB", "NYB", false);
        table_0227.add_value("ORT", "ORT", false);
        table_0227.add_value("OTC", "OTC", false);
        table_0227.add_value("OTH", "OTH", false);
        table_0227.add_value("PD", "PD", false);
        table_0227.add_value("PMC", "PMC", false);
        table_0227.add_value("PRX", "PRX", false);
        table_0227.add_value("PWJ", "PWJ", false);
        table_0227.add_value("SCL", "SCL", false);
        table_0227.add_value("SI", "SI", false);
        table_0227.add_value("SKB", "SKB", false);
        table_0227.add_value("SOL", "SOL", false);
        table_0227.add_value("TAL", "TAL", false);
        table_0227.add_value("UNK", "UNK", false);
        table_0227.add_value("USA", "USA", false);
        table_0227.add_value("VXG", "VXG", false);
        table_0227.add_value("WA", "WA", false);
        table_0227.add_value("WAL", "WAL", false);
        table_0227.add_value("ZLB", "ZLB", false);
        self.add_table(table_0227);

        // HL7 Table 0228 - Diagnosis Classification
        let mut table_0228 = Hl7Table::new(
            "0228",
            "Diagnosis Classification",
            "Diagnosis Classification",
        );
        table_0228.add_value("C", "C", false);
        table_0228.add_value("D", "D", false);
        table_0228.add_value("I", "I", false);
        table_0228.add_value("M", "M", false);
        table_0228.add_value("O", "O", false);
        table_0228.add_value("R", "R", false);
        table_0228.add_value("S", "S", false);
        table_0228.add_value("T", "T", false);
        self.add_table(table_0228);

        // HL7 Table 0229 - DRG Payor
        let mut table_0229 = Hl7Table::new("0229", "DRG Payor", "DRG Payor");
        table_0229.add_value("C", "C", false);
        table_0229.add_value("G", "G", false);
        table_0229.add_value("M", "M", false);
        table_0229.add_value("A", "A", false);
        table_0229.add_value("D", "D", false);
        table_0229.add_value("I", "I", false);
        table_0229.add_value("P", "P", false);
        self.add_table(table_0229);

        // HL7 Table 0230 - Procedure Functional Type
        let mut table_0230 = Hl7Table::new(
            "0230",
            "Procedure Functional Type",
            "Procedure Functional Type",
        );
        table_0230.add_value("A", "A", false);
        table_0230.add_value("D", "D", false);
        table_0230.add_value("I", "I", false);
        table_0230.add_value("P", "P", false);
        self.add_table(table_0230);

        // HL7 Table 0231 - Student Status
        let mut table_0231 = Hl7Table::new("0231", "Student Status", "Student Status");
        table_0231.add_value("F", "F", false);
        table_0231.add_value("N", "N", false);
        table_0231.add_value("P", "P", false);
        self.add_table(table_0231);

        // HL7 Table 0232 - - Insurance Company Contact Reason
        let mut table_0232 = Hl7Table::new(
            "0232",
            "- Insurance Company Contact Reason",
            "- Insurance Company Contact Reason",
        );
        table_0232.add_value("01", "01", false);
        table_0232.add_value("02", "02", false);
        table_0232.add_value("03", "03", false);
        table_0232.add_value("1", "1", false);
        table_0232.add_value("2", "2", false);
        table_0232.add_value("3", "3", false);
        self.add_table(table_0232);

        // HL7 Table 0270 - Document Type
        let mut table_0270 = Hl7Table::new("0270", "Document Type", "Document Type");
        table_0270.add_value("AR", "AR", false);
        table_0270.add_value("CD", "CD", false);
        table_0270.add_value("CN", "CN", false);
        table_0270.add_value("DI", "DI", false);
        table_0270.add_value("DS", "DS", false);
        table_0270.add_value("ED", "ED", false);
        table_0270.add_value("HP", "HP", false);
        table_0270.add_value("OP", "OP", false);
        table_0270.add_value("PC", "PC", false);
        table_0270.add_value("PH", "PH", false);
        table_0270.add_value("PN", "PN", false);
        table_0270.add_value("PR", "PR", false);
        table_0270.add_value("SP", "SP", false);
        table_0270.add_value("TS", "TS", false);
        table_0270.add_value("AU", "AU", false);
        table_0270.add_value("DO", "DO", false);
        table_0270.add_value("IN", "IN", false);
        table_0270.add_value("IP", "IP", false);
        table_0270.add_value("LA", "LA", false);
        table_0270.add_value("PA", "PA", false);
        table_0270.add_value("R", "R", false);
        table_0270.add_value("U", "U", false);
        table_0270.add_value("V", "V", false);
        table_0270.add_value("AV", "AV", false);
        table_0270.add_value("CA", "CA", false);
        table_0270.add_value("OB", "OB", false);
        table_0270.add_value("UN", "UN", false);
        self.add_table(table_0270);

        // HL7 Table 0271 - Document Completion Status
        let mut table_0271 = Hl7Table::new(
            "0271",
            "Document Completion Status",
            "Document Completion Status",
        );
        table_0271.add_value("AU", "AU", false);
        table_0271.add_value("DI", "DI", false);
        table_0271.add_value("DO", "DO", false);
        table_0271.add_value("IN", "IN", false);
        table_0271.add_value("IP", "IP", false);
        table_0271.add_value("LA", "LA", false);
        table_0271.add_value("PA", "PA", false);
        self.add_table(table_0271);

        // HL7 Table 0272 - Document Confidentiality Status
        let mut table_0272 = Hl7Table::new(
            "0272",
            "Document Confidentiality Status",
            "Document Confidentiality Status",
        );
        table_0272.add_value("R", "R", false);
        table_0272.add_value("U", "U", false);
        table_0272.add_value("V", "V", false);
        self.add_table(table_0272);

        // HL7 Table 0273 - Document Availability Status
        let mut table_0273 = Hl7Table::new(
            "0273",
            "Document Availability Status",
            "Document Availability Status",
        );
        table_0273.add_value("AV", "AV", false);
        table_0273.add_value("CA", "CA", false);
        table_0273.add_value("OB", "OB", false);
        table_0273.add_value("UN", "UN", false);
        self.add_table(table_0273);

        // HL7 Table 0275 - Document Storage Status
        let mut table_0275 =
            Hl7Table::new("0275", "Document Storage Status", "Document Storage Status");
        table_0275.add_value("AA", "AA", false);
        table_0275.add_value("AC", "AC", false);
        table_0275.add_value("AR", "AR", false);
        table_0275.add_value("PU", "PU", false);
        table_0275.add_value("CHECKUP", "CHECKUP", false);
        table_0275.add_value("EMERGENCY", "EMERGENCY", false);
        table_0275.add_value("FOLLOWUP", "FOLLOWUP", false);
        table_0275.add_value("ROUTINE", "ROUTINE", false);
        table_0275.add_value("WALKIN", "WALKIN", false);
        self.add_table(table_0275);

        // HL7 Table 0276 - Appointment reason codes
        let mut table_0276 = Hl7Table::new(
            "0276",
            "Appointment reason codes",
            "Appointment reason codes",
        );
        table_0276.add_value("CHECKUP", "CHECKUP", false);
        table_0276.add_value("EMERGENCY", "EMERGENCY", false);
        table_0276.add_value("FOLLOWUP", "FOLLOWUP", false);
        table_0276.add_value("ROUTINE", "ROUTINE", false);
        table_0276.add_value("WALKIN", "WALKIN", false);
        self.add_table(table_0276);

        // HL7 Table 0277 - Appointment Type Codes
        let mut table_0277 =
            Hl7Table::new("0277", "Appointment Type Codes", "Appointment Type Codes");
        table_0277.add_value("Complete", "Complete", false);
        table_0277.add_value("Normal", "Normal", false);
        table_0277.add_value("Tentative", "Tentative", false);
        self.add_table(table_0277);

        // HL7 Table 0278 - Filler status codes
        let mut table_0278 = Hl7Table::new("0278", "Filler status codes", "Filler status codes");
        table_0278.add_value("Blocked", "Blocked", false);
        table_0278.add_value("Booked", "Booked", false);
        table_0278.add_value("Cancelled", "Cancelled", false);
        table_0278.add_value("Complete", "Complete", false);
        table_0278.add_value("Dc", "Dc", false);
        table_0278.add_value("Deleted", "Deleted", false);
        table_0278.add_value("Noshow", "Noshow", false);
        table_0278.add_value("Overbook", "Overbook", false);
        table_0278.add_value("Pending", "Pending", false);
        table_0278.add_value("Started", "Started", false);
        table_0278.add_value("Waitlist", "Waitlist", false);
        table_0278.add_value("Confirm", "Confirm", false);
        table_0278.add_value("No", "No", false);
        table_0278.add_value("Notify", "Notify", false);
        table_0278.add_value("Yes", "Yes", false);
        table_0278.add_value("AD", "AD", false);
        table_0278.add_value("CO", "CO", false);
        table_0278.add_value("DE", "DE", false);
        table_0278.add_value("LI", "LI", false);
        table_0278.add_value("UC", "UC", false);
        table_0278.add_value("UN", "UN", false);
        table_0278.add_value("UP", "UP", false);
        table_0278.add_value("01", "01", false);
        table_0278.add_value("02", "02", false);
        table_0278.add_value("03", "03", false);
        table_0278.add_value("04", "04", false);
        table_0278.add_value("05", "05", false);
        table_0278.add_value("06", "06", false);
        table_0278.add_value("07", "07", false);
        table_0278.add_value("08", "08", false);
        table_0278.add_value("09", "09", false);
        table_0278.add_value("1", "1", false);
        table_0278.add_value("10", "10", false);
        table_0278.add_value("100", "100", false);
        table_0278.add_value("101", "101", false);
        table_0278.add_value("102", "102", false);
        table_0278.add_value("103", "103", false);
        table_0278.add_value("104", "104", false);
        table_0278.add_value("105", "105", false);
        table_0278.add_value("106", "106", false);
        table_0278.add_value("107", "107", false);
        table_0278.add_value("108", "108", false);
        table_0278.add_value("109", "109", false);
        table_0278.add_value("11", "11", false);
        table_0278.add_value("110", "110", false);
        table_0278.add_value("111", "111", false);
        table_0278.add_value("112", "112", false);
        table_0278.add_value("113", "113", false);
        table_0278.add_value("114", "114", false);
        table_0278.add_value("115", "115", false);
        table_0278.add_value("116", "116", false);
        table_0278.add_value("117", "117", false);
        table_0278.add_value("118", "118", false);
        table_0278.add_value("119", "119", false);
        table_0278.add_value("12", "12", false);
        table_0278.add_value("120", "120", false);
        table_0278.add_value("121", "121", false);
        table_0278.add_value("122", "122", false);
        table_0278.add_value("13", "13", false);
        table_0278.add_value("14", "14", false);
        table_0278.add_value("15", "15", false);
        table_0278.add_value("16", "16", false);
        table_0278.add_value("17", "17", false);
        table_0278.add_value("18", "18", false);
        table_0278.add_value("19", "19", false);
        table_0278.add_value("2", "2", false);
        table_0278.add_value("20", "20", false);
        table_0278.add_value("21", "21", false);
        table_0278.add_value("22", "22", false);
        table_0278.add_value("23", "23", false);
        table_0278.add_value("24", "24", false);
        table_0278.add_value("25", "25", false);
        table_0278.add_value("26", "26", false);
        table_0278.add_value("27", "27", false);
        table_0278.add_value("28", "28", false);
        table_0278.add_value("29", "29", false);
        table_0278.add_value("3", "3", false);
        table_0278.add_value("30", "30", false);
        table_0278.add_value("31", "31", false);
        table_0278.add_value("32", "32", false);
        table_0278.add_value("33", "33", false);
        table_0278.add_value("34", "34", false);
        table_0278.add_value("35", "35", false);
        table_0278.add_value("36", "36", false);
        table_0278.add_value("37", "37", false);
        table_0278.add_value("38", "38", false);
        table_0278.add_value("39", "39", false);
        table_0278.add_value("4", "4", false);
        table_0278.add_value("40", "40", false);
        table_0278.add_value("41", "41", false);
        table_0278.add_value("42", "42", false);
        table_0278.add_value("43", "43", false);
        table_0278.add_value("44", "44", false);
        table_0278.add_value("45", "45", false);
        table_0278.add_value("46", "46", false);
        table_0278.add_value("47", "47", false);
        table_0278.add_value("48", "48", false);
        table_0278.add_value("49", "49", false);
        table_0278.add_value("5", "5", false);
        table_0278.add_value("50", "50", false);
        table_0278.add_value("51", "51", false);
        table_0278.add_value("52", "52", false);
        table_0278.add_value("53", "53", false);
        table_0278.add_value("54", "54", false);
        table_0278.add_value("55", "55", false);
        table_0278.add_value("56", "56", false);
        table_0278.add_value("57", "57", false);
        table_0278.add_value("58", "58", false);
        table_0278.add_value("59", "59", false);
        table_0278.add_value("6", "6", false);
        table_0278.add_value("60", "60", false);
        table_0278.add_value("61", "61", false);
        table_0278.add_value("62", "62", false);
        table_0278.add_value("63", "63", false);
        table_0278.add_value("64", "64", false);
        table_0278.add_value("65", "65", false);
        table_0278.add_value("66", "66", false);
        table_0278.add_value("67", "67", false);
        table_0278.add_value("68", "68", false);
        table_0278.add_value("69", "69", false);
        table_0278.add_value("7", "7", false);
        table_0278.add_value("70", "70", false);
        table_0278.add_value("71", "71", false);
        table_0278.add_value("72", "72", false);
        table_0278.add_value("73", "73", false);
        table_0278.add_value("74", "74", false);
        table_0278.add_value("75", "75", false);
        table_0278.add_value("76", "76", false);
        table_0278.add_value("77", "77", false);
        table_0278.add_value("78", "78", false);
        table_0278.add_value("79", "79", false);
        table_0278.add_value("8", "8", false);
        table_0278.add_value("80", "80", false);
        table_0278.add_value("81", "81", false);
        table_0278.add_value("82", "82", false);
        table_0278.add_value("83", "83", false);
        table_0278.add_value("84", "84", false);
        table_0278.add_value("85", "85", false);
        table_0278.add_value("86", "86", false);
        table_0278.add_value("87", "87", false);
        table_0278.add_value("88", "88", false);
        table_0278.add_value("89", "89", false);
        table_0278.add_value("9", "9", false);
        table_0278.add_value("90", "90", false);
        table_0278.add_value("91", "91", false);
        table_0278.add_value("92", "92", false);
        table_0278.add_value("93", "93", false);
        table_0278.add_value("94", "94", false);
        table_0278.add_value("95", "95", false);
        table_0278.add_value("96", "96", false);
        table_0278.add_value("97", "97", false);
        table_0278.add_value("98", "98", false);
        table_0278.add_value("99", "99", false);
        table_0278.add_value("998", "998", false);
        table_0278.add_value("999", "999", false);
        self.add_table(table_0278);

        // HL7 Table 0279 - Allow Substitution Codes
        let mut table_0279 = Hl7Table::new(
            "0279",
            "Allow Substitution Codes",
            "Allow Substitution Codes",
        );
        table_0279.add_value("Confirm", "Confirm", false);
        table_0279.add_value("No", "No", false);
        table_0279.add_value("Notify", "Notify", false);
        table_0279.add_value("Yes", "Yes", false);
        self.add_table(table_0279);

        // HL7 Table 0280 - Referral Priority
        let mut table_0280 = Hl7Table::new("0280", "Referral Priority", "Referral Priority");
        table_0280.add_value("A", "A", false);
        table_0280.add_value("R", "R", false);
        table_0280.add_value("S", "S", false);
        self.add_table(table_0280);

        // HL7 Table 0281 - Referral Type
        let mut table_0281 = Hl7Table::new("0281", "Referral Type", "Referral Type");
        table_0281.add_value("Hom", "Hom", false);
        table_0281.add_value("Lab", "Lab", false);
        table_0281.add_value("Med", "Med", false);
        table_0281.add_value("Psy", "Psy", false);
        table_0281.add_value("Rad", "Rad", false);
        table_0281.add_value("Skn", "Skn", false);
        self.add_table(table_0281);

        // HL7 Table 0282 - Referral Disposition
        let mut table_0282 = Hl7Table::new("0282", "Referral Disposition", "Referral Disposition");
        table_0282.add_value("AM", "AM", false);
        table_0282.add_value("RP", "RP", false);
        table_0282.add_value("SO", "SO", false);
        table_0282.add_value("WR", "WR", false);
        self.add_table(table_0282);

        // HL7 Table 0283 - Referral Status
        let mut table_0283 = Hl7Table::new("0283", "Referral Status", "Referral Status");
        table_0283.add_value("A", "A", false);
        table_0283.add_value("E", "E", false);
        table_0283.add_value("P", "P", false);
        table_0283.add_value("R", "R", false);
        self.add_table(table_0283);

        // HL7 Table 0284 - Referral Category
        let mut table_0284 = Hl7Table::new("0284", "Referral Category", "Referral Category");
        table_0284.add_value("A", "A", false);
        table_0284.add_value("E", "E", false);
        table_0284.add_value("I", "I", false);
        table_0284.add_value("O", "O", false);
        self.add_table(table_0284);

        // HL7 Table 0287 - Problem/Goal Action Code
        let mut table_0287 = Hl7Table::new(
            "0287",
            "Problem/Goal Action Code",
            "Problem/Goal Action Code",
        );
        table_0287.add_value("AD", "AD", false);
        table_0287.add_value("CO", "CO", false);
        table_0287.add_value("DE", "DE", false);
        table_0287.add_value("LI", "LI", false);
        table_0287.add_value("UC", "UC", false);
        table_0287.add_value("UN", "UN", false);
        table_0287.add_value("UP", "UP", false);
        self.add_table(table_0287);

        // HL7 Table 0292 - Vaccines Administered (code = CVX) (parenteral, unless oral is noted)
        let mut table_0292 = Hl7Table::new(
            "0292",
            "Vaccines Administered (code = CVX) (parenteral, unless oral is noted)",
            "Vaccines Administered (code = CVX) (parenteral, unless oral is noted)",
        );
        table_0292.add_value("01", "01", false);
        table_0292.add_value("02", "02", false);
        table_0292.add_value("03", "03", false);
        table_0292.add_value("04", "04", false);
        table_0292.add_value("05", "05", false);
        table_0292.add_value("06", "06", false);
        table_0292.add_value("07", "07", false);
        table_0292.add_value("08", "08", false);
        table_0292.add_value("09", "09", false);
        table_0292.add_value("1", "1", false);
        table_0292.add_value("10", "10", false);
        table_0292.add_value("100", "100", false);
        table_0292.add_value("101", "101", false);
        table_0292.add_value("102", "102", false);
        table_0292.add_value("103", "103", false);
        table_0292.add_value("104", "104", false);
        table_0292.add_value("105", "105", false);
        table_0292.add_value("106", "106", false);
        table_0292.add_value("107", "107", false);
        table_0292.add_value("108", "108", false);
        table_0292.add_value("109", "109", false);
        table_0292.add_value("11", "11", false);
        table_0292.add_value("110", "110", false);
        table_0292.add_value("111", "111", false);
        table_0292.add_value("112", "112", false);
        table_0292.add_value("113", "113", false);
        table_0292.add_value("114", "114", false);
        table_0292.add_value("115", "115", false);
        table_0292.add_value("116", "116", false);
        table_0292.add_value("117", "117", false);
        table_0292.add_value("118", "118", false);
        table_0292.add_value("119", "119", false);
        table_0292.add_value("12", "12", false);
        table_0292.add_value("120", "120", false);
        table_0292.add_value("121", "121", false);
        table_0292.add_value("122", "122", false);
        table_0292.add_value("13", "13", false);
        table_0292.add_value("14", "14", false);
        table_0292.add_value("15", "15", false);
        table_0292.add_value("16", "16", false);
        table_0292.add_value("17", "17", false);
        table_0292.add_value("18", "18", false);
        table_0292.add_value("19", "19", false);
        table_0292.add_value("2", "2", false);
        table_0292.add_value("20", "20", false);
        table_0292.add_value("21", "21", false);
        table_0292.add_value("22", "22", false);
        table_0292.add_value("23", "23", false);
        table_0292.add_value("24", "24", false);
        table_0292.add_value("25", "25", false);
        table_0292.add_value("26", "26", false);
        table_0292.add_value("27", "27", false);
        table_0292.add_value("28", "28", false);
        table_0292.add_value("29", "29", false);
        table_0292.add_value("3", "3", false);
        table_0292.add_value("30", "30", false);
        table_0292.add_value("31", "31", false);
        table_0292.add_value("32", "32", false);
        table_0292.add_value("33", "33", false);
        table_0292.add_value("34", "34", false);
        table_0292.add_value("35", "35", false);
        table_0292.add_value("36", "36", false);
        table_0292.add_value("37", "37", false);
        table_0292.add_value("38", "38", false);
        table_0292.add_value("39", "39", false);
        table_0292.add_value("4", "4", false);
        table_0292.add_value("40", "40", false);
        table_0292.add_value("41", "41", false);
        table_0292.add_value("42", "42", false);
        table_0292.add_value("43", "43", false);
        table_0292.add_value("44", "44", false);
        table_0292.add_value("45", "45", false);
        table_0292.add_value("46", "46", false);
        table_0292.add_value("47", "47", false);
        table_0292.add_value("48", "48", false);
        table_0292.add_value("49", "49", false);
        table_0292.add_value("5", "5", false);
        table_0292.add_value("50", "50", false);
        table_0292.add_value("51", "51", false);
        table_0292.add_value("52", "52", false);
        table_0292.add_value("53", "53", false);
        table_0292.add_value("54", "54", false);
        table_0292.add_value("55", "55", false);
        table_0292.add_value("56", "56", false);
        table_0292.add_value("57", "57", false);
        table_0292.add_value("58", "58", false);
        table_0292.add_value("59", "59", false);
        table_0292.add_value("6", "6", false);
        table_0292.add_value("60", "60", false);
        table_0292.add_value("61", "61", false);
        table_0292.add_value("62", "62", false);
        table_0292.add_value("63", "63", false);
        table_0292.add_value("64", "64", false);
        table_0292.add_value("65", "65", false);
        table_0292.add_value("66", "66", false);
        table_0292.add_value("67", "67", false);
        table_0292.add_value("68", "68", false);
        table_0292.add_value("69", "69", false);
        table_0292.add_value("7", "7", false);
        table_0292.add_value("70", "70", false);
        table_0292.add_value("71", "71", false);
        table_0292.add_value("72", "72", false);
        table_0292.add_value("73", "73", false);
        table_0292.add_value("74", "74", false);
        table_0292.add_value("75", "75", false);
        table_0292.add_value("76", "76", false);
        table_0292.add_value("77", "77", false);
        table_0292.add_value("78", "78", false);
        table_0292.add_value("79", "79", false);
        table_0292.add_value("8", "8", false);
        table_0292.add_value("80", "80", false);
        table_0292.add_value("81", "81", false);
        table_0292.add_value("82", "82", false);
        table_0292.add_value("83", "83", false);
        table_0292.add_value("84", "84", false);
        table_0292.add_value("85", "85", false);
        table_0292.add_value("86", "86", false);
        table_0292.add_value("87", "87", false);
        table_0292.add_value("88", "88", false);
        table_0292.add_value("89", "89", false);
        table_0292.add_value("9", "9", false);
        table_0292.add_value("90", "90", false);
        table_0292.add_value("91", "91", false);
        table_0292.add_value("92", "92", false);
        table_0292.add_value("93", "93", false);
        table_0292.add_value("94", "94", false);
        table_0292.add_value("95", "95", false);
        table_0292.add_value("96", "96", false);
        table_0292.add_value("97", "97", false);
        table_0292.add_value("98", "98", false);
        table_0292.add_value("99", "99", false);
        table_0292.add_value("998", "998", false);
        table_0292.add_value("999", "999", false);
        self.add_table(table_0292);

        // HL7 Table 0298 - CP Range Type
        let mut table_0298 = Hl7Table::new("0298", "CP Range Type", "CP Range Type");
        table_0298.add_value("F", "F", false);
        table_0298.add_value("P", "P", false);
        self.add_table(table_0298);

        // HL7 Table 0301 - Universal ID Type
        let mut table_0301 =
            Hl7Table::new("0301", "Universal ID Type", "Type of universal identifier");
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
        table_0301.add_value("CLIA", "CLIA", false);
        table_0301.add_value("CLIP", "CLIP", false);
        table_0301.add_value("EUI64", "EUI64", false);
        self.add_table(table_0301);

        // HL7 Table 0305 - Person Location Type
        let mut table_0305 = Hl7Table::new("0305", "Person Location Type", "Person Location Type");
        table_0305.add_value("C", "C", false);
        table_0305.add_value("D", "D", false);
        table_0305.add_value("H", "H", false);
        table_0305.add_value("N", "N", false);
        table_0305.add_value("O", "O", false);
        table_0305.add_value("P", "P", false);
        table_0305.add_value("S", "S", false);
        self.add_table(table_0305);

        // HL7 Table 0309 - Coverage Type
        let mut table_0309 = Hl7Table::new("0309", "Coverage Type", "Coverage Type");
        table_0309.add_value("B", "B", false);
        table_0309.add_value("H", "H", false);
        table_0309.add_value("P", "P", false);
        table_0309.add_value("RX", "RX", false);
        self.add_table(table_0309);

        // HL7 Table 0311 - Job Status
        let mut table_0311 = Hl7Table::new("0311", "Job Status", "Job Status");
        table_0311.add_value("O", "O", false);
        table_0311.add_value("P", "P", false);
        table_0311.add_value("T", "T", false);
        table_0311.add_value("U", "U", false);
        self.add_table(table_0311);

        // HL7 Table 0315 - Living Will Code
        let mut table_0315 = Hl7Table::new("0315", "Living Will Code", "Living Will Code");
        table_0315.add_value("F", "F", false);
        table_0315.add_value("I", "I", false);
        table_0315.add_value("N", "N", false);
        table_0315.add_value("U", "U", false);
        table_0315.add_value("Y", "Y", false);
        self.add_table(table_0315);

        // HL7 Table 0316 - Organ Donor Code
        let mut table_0316 = Hl7Table::new("0316", "Organ Donor Code", "Organ Donor Code");
        table_0316.add_value("F", "F", false);
        table_0316.add_value("I", "I", false);
        table_0316.add_value("N", "N", false);
        table_0316.add_value("P", "P", false);
        table_0316.add_value("R", "R", false);
        table_0316.add_value("U", "U", false);
        table_0316.add_value("Y", "Y", false);
        self.add_table(table_0316);

        // HL7 Table 0321 - Dispense Method
        let mut table_0321 = Hl7Table::new("0321", "Dispense Method", "Dispense Method");
        table_0321.add_value("AD", "AD", false);
        table_0321.add_value("F", "F", false);
        table_0321.add_value("TR", "TR", false);
        table_0321.add_value("UD", "UD", false);
        self.add_table(table_0321);

        // HL7 Table 0322 - Completion Status
        let mut table_0322 = Hl7Table::new("0322", "Completion Status", "Completion Status");
        table_0322.add_value("CP", "CP", false);
        table_0322.add_value("NA", "NA", false);
        table_0322.add_value("PA", "PA", false);
        table_0322.add_value("RE", "RE", false);
        self.add_table(table_0322);

        // HL7 Table 0323 - Action Code
        let mut table_0323 = Hl7Table::new("0323", "Action Code", "Action Code");
        table_0323.add_value("A", "A", false);
        table_0323.add_value("D", "D", false);
        table_0323.add_value("U", "U", false);
        table_0323.add_value("X", "X", false);
        self.add_table(table_0323);

        // HL7 Table 0326 - Visit Indicator
        let mut table_0326 = Hl7Table::new("0326", "Visit Indicator", "Visit Indicator");
        table_0326.add_value("A", "A", false);
        table_0326.add_value("V", "V", false);
        self.add_table(table_0326);

        // HL7 Table 0333 - Driver's License Issuing Authority
        let mut table_0333 = Hl7Table::new(
            "0333",
            "Driver's License Issuing Authority",
            "Driver's License Issuing Authority",
        );
        table_0333.add_value("M", "M", false);
        table_0333.add_value("SD", "SD", false);
        table_0333.add_value("SU", "SU", false);
        self.add_table(table_0333);

        // HL7 Table 0334 - Disabled Person Code
        let mut table_0334 = Hl7Table::new("0334", "Disabled Person Code", "Disabled Person Code");
        table_0334.add_value("AP", "AP", false);
        table_0334.add_value("GT", "GT", false);
        table_0334.add_value("IN", "IN", false);
        table_0334.add_value("PT", "PT", false);
        table_0334.add_value("1", "1", false);
        table_0334.add_value("2", "2", false);
        table_0334.add_value("3", "3", false);
        table_0334.add_value("4", "4", false);
        self.add_table(table_0334);

        // HL7 Table 0335 - Repeat Pattern
        let mut table_0335 = Hl7Table::new("0335", "Repeat Pattern", "Repeat Pattern");
        table_0335.add_value("A", "A", false);
        table_0335.add_value("BID", "BID", false);
        table_0335.add_value("C", "C", false);
        table_0335.add_value("D", "D", false);
        table_0335.add_value("I", "I", false);
        table_0335.add_value("M", "M", false);
        table_0335.add_value("Once", "Once", false);
        table_0335.add_value("P", "P", false);
        table_0335.add_value("PRN", "PRN", false);
        table_0335.add_value("QAM", "QAM", false);
        table_0335.add_value("QHS", "QHS", false);
        table_0335.add_value("QID", "QID", false);
        table_0335.add_value("QOD", "QOD", false);
        table_0335.add_value("QPM", "QPM", false);
        table_0335.add_value("QSHIFT", "QSHIFT", false);
        table_0335.add_value("TID", "TID", false);
        table_0335.add_value("V", "V", false);
        table_0335.add_pattern("PRNxxx");
        table_0335.add_pattern("Q<integer>D");
        table_0335.add_pattern("Q<integer>H");
        table_0335.add_pattern("Q<integer>J<day#>");
        table_0335.add_pattern("Q<integer>L");
        table_0335.add_pattern("Q<integer>M");
        table_0335.add_pattern("Q<integer>S");
        table_0335.add_pattern("Q<integer>W");
        table_0335.add_pattern("xID");
        self.add_table(table_0335);

        // HL7 Table 0336 - Referral Reason
        let mut table_0336 = Hl7Table::new("0336", "Referral Reason", "Referral Reason");
        table_0336.add_value("O", "O", false);
        table_0336.add_value("P", "P", false);
        table_0336.add_value("S", "S", false);
        table_0336.add_value("W", "W", false);
        self.add_table(table_0336);

        // HL7 Table 0338 - Practitioner ID Number Type
        let mut table_0338 = Hl7Table::new(
            "0338",
            "Practitioner ID Number Type",
            "Practitioner ID Number Type",
        );
        table_0338.add_value("CY", "CY", false);
        table_0338.add_value("DEA", "DEA", false);
        table_0338.add_value("GL", "GL", false);
        table_0338.add_value("L&I", "L&I", false);
        table_0338.add_value("LI", "LI", false);
        table_0338.add_value("MCD", "MCD", false);
        table_0338.add_value("MCR", "MCR", false);
        table_0338.add_value("QA", "QA", false);
        table_0338.add_value("SL", "SL", false);
        table_0338.add_value("TAX", "TAX", false);
        table_0338.add_value("TRL", "TRL", false);
        table_0338.add_value("UPIN", "UPIN", false);
        self.add_table(table_0338);

        // HL7 Table 0339 - Advanced Beneficiary Notice Code
        let mut table_0339 = Hl7Table::new(
            "0339",
            "Advanced Beneficiary Notice Code",
            "Advanced Beneficiary Notice Code",
        );
        table_0339.add_value("1", "1", false);
        table_0339.add_value("2", "2", false);
        table_0339.add_value("3", "3", false);
        table_0339.add_value("4", "4", false);
        self.add_table(table_0339);

        // HL7 Table 0340 - Procedure Code Modifier
        let mut table_0340 =
            Hl7Table::new("0340", "Procedure Code Modifier", "Procedure Code Modifier");
        table_0340.add_value("CPTM", "CPTM", false);
        table_0340.add_value("HPC", "HPC", false);
        self.add_table(table_0340);

        // HL7 Table 0344 - Patient's Relationship to Insured
        let mut table_0344 = Hl7Table::new(
            "0344",
            "Patient's Relationship to Insured",
            "Patient's Relationship to Insured",
        );
        table_0344.add_value("01", "01", false);
        table_0344.add_value("02", "02", false);
        table_0344.add_value("03", "03", false);
        table_0344.add_value("04", "04", false);
        table_0344.add_value("05", "05", false);
        table_0344.add_value("06", "06", false);
        table_0344.add_value("07", "07", false);
        table_0344.add_value("08", "08", false);
        table_0344.add_value("09", "09", false);
        table_0344.add_value("1", "1", false);
        table_0344.add_value("10", "10", false);
        table_0344.add_value("11", "11", false);
        table_0344.add_value("12", "12", false);
        table_0344.add_value("13", "13", false);
        table_0344.add_value("14", "14", false);
        table_0344.add_value("15", "15", false);
        table_0344.add_value("16", "16", false);
        table_0344.add_value("17", "17", false);
        table_0344.add_value("18", "18", false);
        table_0344.add_value("19", "19", false);
        table_0344.add_value("2", "2", false);
        table_0344.add_value("3", "3", false);
        table_0344.add_value("4", "4", false);
        table_0344.add_value("5", "5", false);
        table_0344.add_value("6", "6", false);
        table_0344.add_value("7", "7", false);
        table_0344.add_value("8", "8", false);
        table_0344.add_value("9", "9", false);
        self.add_table(table_0344);

        // HL7 Table 0347 - State/Province
        let mut table_0347 = Hl7Table::new("0347", "State/Province", "State/Province");
        table_0347.add_value("AB", "AB", false);
        table_0347.add_value("MI", "MI", false);
        self.add_table(table_0347);

        // HL7 Table 0348 - Special program indicator
        let mut table_0348 = Hl7Table::new(
            "0348",
            "Special program indicator",
            "Special program indicator",
        );
        table_0348.add_value("01", "01", false);
        table_0348.add_value("02", "02", false);
        table_0348.add_value("03", "03", false);
        table_0348.add_value("04", "04", false);
        table_0348.add_value("05", "05", false);
        table_0348.add_value("06", "06", false);
        table_0348.add_value("07", "07", false);
        table_0348.add_value("08", "08", false);
        self.add_table(table_0348);

        // HL7 Table 0349 - PSRO/UR approval indicator
        let mut table_0349 = Hl7Table::new(
            "0349",
            "PSRO/UR approval indicator",
            "PSRO/UR approval indicator",
        );
        table_0349.add_value("1", "1", false);
        table_0349.add_value("2", "2", false);
        table_0349.add_value("3", "3", false);
        table_0349.add_value("4", "4", false);
        table_0349.add_value("5", "5", false);
        self.add_table(table_0349);

        // HL7 Table 0350 - Occurrence Code
        let mut table_0350 = Hl7Table::new("0350", "Occurrence Code", "Occurrence Code");
        table_0350.add_value("01", "01", false);
        table_0350.add_value("02", "02", false);
        table_0350.add_value("03", "03", false);
        table_0350.add_value("04", "04", false);
        table_0350.add_value("05", "05", false);
        table_0350.add_value("06", "06", false);
        table_0350.add_value("09", "09", false);
        table_0350.add_value("10", "10", false);
        table_0350.add_value("11", "11", false);
        table_0350.add_value("12", "12", false);
        table_0350.add_value("17", "17", false);
        table_0350.add_value("18", "18", false);
        table_0350.add_value("19", "19", false);
        table_0350.add_value("20", "20", false);
        table_0350.add_value("21", "21", false);
        table_0350.add_value("22", "22", false);
        table_0350.add_value("24", "24", false);
        table_0350.add_value("25", "25", false);
        table_0350.add_value("26", "26", false);
        table_0350.add_value("27", "27", false);
        table_0350.add_value("28", "28", false);
        table_0350.add_value("29", "29", false);
        table_0350.add_value("30", "30", false);
        table_0350.add_value("31", "31", false);
        table_0350.add_value("32", "32", false);
        table_0350.add_value("33", "33", false);
        table_0350.add_value("34", "34", false);
        table_0350.add_value("35", "35", false);
        table_0350.add_value("36", "36", false);
        table_0350.add_value("37", "37", false);
        table_0350.add_value("40", "40", false);
        table_0350.add_value("41", "41", false);
        table_0350.add_value("42", "42", false);
        table_0350.add_value("43", "43", false);
        table_0350.add_value("44", "44", false);
        table_0350.add_value("45", "45", false);
        table_0350.add_value("46", "46", false);
        table_0350.add_value("47", "47", false);
        table_0350.add_value("48", "48", false);
        table_0350.add_value("49", "49", false);
        table_0350.add_value("50", "50", false);
        table_0350.add_value("51", "51", false);
        table_0350.add_value("70", "70", false);
        table_0350.add_value("71", "71", false);
        table_0350.add_value("72", "72", false);
        table_0350.add_value("73", "73", false);
        table_0350.add_value("74", "74", false);
        table_0350.add_value("75", "75", false);
        table_0350.add_value("76", "76", false);
        table_0350.add_value("77", "77", false);
        table_0350.add_value("78", "78", false);
        table_0350.add_value("79", "79", false);
        table_0350.add_value("80", "80", false);
        table_0350.add_value("81", "81", false);
        table_0350.add_value("82", "82", false);
        table_0350.add_value("83", "83", false);
        table_0350.add_value("84", "84", false);
        table_0350.add_value("85", "85", false);
        table_0350.add_value("86", "86", false);
        table_0350.add_value("87", "87", false);
        table_0350.add_value("88", "88", false);
        table_0350.add_value("89", "89", false);
        table_0350.add_value("90", "90", false);
        table_0350.add_value("91", "91", false);
        table_0350.add_value("92", "92", false);
        table_0350.add_value("93", "93", false);
        table_0350.add_value("94", "94", false);
        table_0350.add_value("95", "95", false);
        table_0350.add_value("96", "96", false);
        table_0350.add_value("97", "97", false);
        table_0350.add_value("98", "98", false);
        table_0350.add_value("99", "99", false);
        table_0350.add_value("A1", "A1", false);
        table_0350.add_value("A2", "A2", false);
        table_0350.add_value("A3", "A3", false);
        self.add_table(table_0350);

        // HL7 Table 0351 - Occurrence Span
        let mut table_0351 = Hl7Table::new("0351", "Occurrence Span", "Occurrence Span");
        table_0351.add_value("70", "70", false);
        table_0351.add_value("71", "71", false);
        table_0351.add_value("72", "72", false);
        table_0351.add_value("73", "73", false);
        table_0351.add_value("74", "74", false);
        table_0351.add_value("75", "75", false);
        table_0351.add_value("76", "76", false);
        table_0351.add_value("77", "77", false);
        table_0351.add_value("78", "78", false);
        table_0351.add_value("79", "79", false);
        table_0351.add_value("M0", "M0", false);
        self.add_table(table_0351);

        // HL7 Table 0354 - Message Structure
        let mut table_0354 = Hl7Table::new("0354", "Message Structure", "Message Structure");
        table_0354.add_value("ACK", "ACK", false);
        table_0354.add_value("ADR_A19", "ADR_A19", false);
        table_0354.add_value("ADT_A01", "ADT_A01", false);
        table_0354.add_value("ADT_A02", "ADT_A02", false);
        table_0354.add_value("ADT_A03", "ADT_A03", false);
        table_0354.add_value("ADT_A05", "ADT_A05", false);
        table_0354.add_value("ADT_A06", "ADT_A06", false);
        table_0354.add_value("ADT_A09", "ADT_A09", false);
        table_0354.add_value("ADT_A12", "ADT_A12", false);
        table_0354.add_value("ADT_A15", "ADT_A15", false);
        table_0354.add_value("ADT_A16", "ADT_A16", false);
        table_0354.add_value("ADT_A17", "ADT_A17", false);
        table_0354.add_value("ADT_A18", "ADT_A18", false);
        table_0354.add_value("ADT_A20", "ADT_A20", false);
        table_0354.add_value("ADT_A21", "ADT_A21", false);
        table_0354.add_value("ADT_A24", "ADT_A24", false);
        table_0354.add_value("ADT_A28", "ADT_A28", false);
        table_0354.add_value("ADT_A30", "ADT_A30", false);
        table_0354.add_value("ADT_A37", "ADT_A37", false);
        table_0354.add_value("ADT_A38", "ADT_A38", false);
        table_0354.add_value("ADT_A39", "ADT_A39", false);
        table_0354.add_value("ADT_A43", "ADT_A43", false);
        table_0354.add_value("ADT_A44", "ADT_A44", false);
        table_0354.add_value("ADT_A45", "ADT_A45", false);
        table_0354.add_value("ADT_A50", "ADT_A50", false);
        table_0354.add_value("ADT_A52", "ADT_A52", false);
        table_0354.add_value("ADT_A54", "ADT_A54", false);
        table_0354.add_value("ADT_A60", "ADT_A60", false);
        table_0354.add_value("ADT_A61", "ADT_A61", false);
        table_0354.add_value("ARD_A19", "ARD_A19", false);
        table_0354.add_value("BAR_P01", "BAR_P01", false);
        table_0354.add_value("BAR_P02", "BAR_P02", false);
        table_0354.add_value("BAR_P05", "BAR_P05", false);
        table_0354.add_value("BAR_P06", "BAR_P06", false);
        table_0354.add_value("BAR_P10", "BAR_P10", false);
        table_0354.add_value("BAR_P12", "BAR_P12", false);
        table_0354.add_value("BPS_O29", "BPS_O29", false);
        table_0354.add_value("BRP_030", "BRP_030", false);
        table_0354.add_value("BRP_O30", "BRP_O30", false);
        table_0354.add_value("BRT_O32", "BRT_O32", false);
        table_0354.add_value("BTS_O31", "BTS_O31", false);
        table_0354.add_value("CCF_I22", "CCF_I22", false);
        table_0354.add_value("CCI_I22", "CCI_I22", false);
        table_0354.add_value("CCM_I21", "CCM_I21", false);
        table_0354.add_value("CCQ_I19", "CCQ_I19", false);
        table_0354.add_value("CCR_I16", "CCR_I16", false);
        table_0354.add_value("CCU_I20", "CCU_I20", false);
        table_0354.add_value("CQU_I19", "CQU_I19", false);
        table_0354.add_value("CRM_C01", "CRM_C01", false);
        table_0354.add_value("CSU_C09", "CSU_C09", false);
        table_0354.add_value("DBC_O41", "DBC_O41", false);
        table_0354.add_value("DBC_O42", "DBC_O42", false);
        table_0354.add_value("DEL_O46", "DEL_O46", false);
        table_0354.add_value("DEO_O45", "DEO_O45", false);
        table_0354.add_value("DER_O44", "DER_O44", false);
        table_0354.add_value("DFT_P03", "DFT_P03", false);
        table_0354.add_value("DFT_P11", "DFT_P11", false);
        table_0354.add_value("DOC_T12", "DOC_T12", false);
        table_0354.add_value("DPR_O48", "DPR_O48", false);
        table_0354.add_value("DRC_O47", "DRC_O47", false);
        table_0354.add_value("DRG_O43", "DRG_O43", false);
        table_0354.add_value("DSR_P04", "DSR_P04", false);
        table_0354.add_value("DSR_Q01", "DSR_Q01", false);
        table_0354.add_value("DSR_Q03", "DSR_Q03", false);
        table_0354.add_value("EAC_U07", "EAC_U07", false);
        table_0354.add_value("EAN_U09", "EAN_U09", false);
        table_0354.add_value("EAR_U08", "EAR_U08", false);
        table_0354.add_value("EDR_R07", "EDR_R07", false);
        table_0354.add_value("EHC_E01", "EHC_E01", false);
        table_0354.add_value("EHC_E02", "EHC_E02", false);
        table_0354.add_value("EHC_E04", "EHC_E04", false);
        table_0354.add_value("EHC_E10", "EHC_E10", false);
        table_0354.add_value("EHC_E12", "EHC_E12", false);
        table_0354.add_value("EHC_E13", "EHC_E13", false);
        table_0354.add_value("EHC_E15", "EHC_E15", false);
        table_0354.add_value("EHC_E20", "EHC_E20", false);
        table_0354.add_value("EHC_E21", "EHC_E21", false);
        table_0354.add_value("EHC_E24", "EHC_E24", false);
        table_0354.add_value("EQQ_Q04", "EQQ_Q04", false);
        table_0354.add_value("ERP_R09", "ERP_R09", false);
        table_0354.add_value("ESR_U02", "ESR_U02", false);
        table_0354.add_value("ESU_U01", "ESU_U01", false);
        table_0354.add_value("INR_U06", "INR_U06", false);
        table_0354.add_value("INU_U05", "INU_U05", false);
        table_0354.add_value("LSU_U12", "LSU_U12", false);
        table_0354.add_value("MDM_T01", "MDM_T01", false);
        table_0354.add_value("MDM_T02", "MDM_T02", false);
        table_0354.add_value("MFD_MFA", "MFD_MFA", false);
        table_0354.add_value("MFD_P09", "MFD_P09", false);
        table_0354.add_value("MFK_M01", "MFK_M01", false);
        table_0354.add_value("MFN_M01", "MFN_M01", false);
        table_0354.add_value("MFN_M02", "MFN_M02", false);
        table_0354.add_value("MFN_M03", "MFN_M03", false);
        table_0354.add_value("MFN_M04", "MFN_M04", false);
        table_0354.add_value("MFN_M05", "MFN_M05", false);
        table_0354.add_value("MFN_M06", "MFN_M06", false);
        table_0354.add_value("MFN_M07", "MFN_M07", false);
        table_0354.add_value("MFN_M08", "MFN_M08", false);
        table_0354.add_value("MFN_M09", "MFN_M09", false);
        table_0354.add_value("MFN_M10", "MFN_M10", false);
        table_0354.add_value("MFN_M11", "MFN_M11", false);
        table_0354.add_value("MFN_M12", "MFN_M12", false);
        table_0354.add_value("MFN_M13", "MFN_M13", false);
        table_0354.add_value("MFN_M15", "MFN_M15", false);
        table_0354.add_value("MFN_M16", "MFN_M16", false);
        table_0354.add_value("MFN_M17", "MFN_M17", false);
        table_0354.add_value("MFN_M18", "MFN_M18", false);
        table_0354.add_value("MFN_Znn", "MFN_Znn", false);
        table_0354.add_value("MFQ_M01", "MFQ_M01", false);
        table_0354.add_value("MFR_M01", "MFR_M01", false);
        table_0354.add_value("MFR_M04", "MFR_M04", false);
        table_0354.add_value("MFR_M05", "MFR_M05", false);
        table_0354.add_value("MFR_M06", "MFR_M06", false);
        table_0354.add_value("MFR_M07", "MFR_M07", false);
        table_0354.add_value("NMD_N02", "NMD_N02", false);
        table_0354.add_value("NMQ_N01", "NMQ_N01", false);
        table_0354.add_value("NMR_N01", "NMR_N01", false);
        table_0354.add_value("NUL", "NUL", false);
        table_0354.add_value("OMB_O27", "OMB_O27", false);
        table_0354.add_value("OMD_O01", "OMD_O01", false);
        table_0354.add_value("OMD_O03", "OMD_O03", false);
        table_0354.add_value("OMG_O19", "OMG_O19", false);
        table_0354.add_value("OMI_O23", "OMI_O23", false);
        table_0354.add_value("OML_O21", "OML_O21", false);
        table_0354.add_value("OML_O33", "OML_O33", false);
        table_0354.add_value("OML_O35", "OML_O35", false);
        table_0354.add_value("OML_O39", "OML_O39", false);
        table_0354.add_value("OMN_O01", "OMN_O01", false);
        table_0354.add_value("OMN_O07", "OMN_O07", false);
        table_0354.add_value("OMP_O09", "OMP_O09", false);
        table_0354.add_value("OMQ_O42", "OMQ_O42", false);
        table_0354.add_value("OMS_O01", "OMS_O01", false);
        table_0354.add_value("OMS_O05", "OMS_O05", false);
        table_0354.add_value("OPL_O37", "OPL_O37", false);
        table_0354.add_value("OPR_O38", "OPR_O38", false);
        table_0354.add_value("OPU_R25", "OPU_R25", false);
        table_0354.add_value("ORA_R33", "ORA_R33", false);
        table_0354.add_value("ORA_R41", "ORA_R41", false);
        table_0354.add_value("ORB_O28", "ORB_O28", false);
        table_0354.add_value("ORD_O02", "ORD_O02", false);
        table_0354.add_value("ORD_O04", "ORD_O04", false);
        table_0354.add_value("ORF_R02", "ORF_R02", false);
        table_0354.add_value("ORF_R04", "ORF_R04", false);
        table_0354.add_value("ORG_O20", "ORG_O20", false);
        table_0354.add_value("ORI_O24", "ORI_O24", false);
        table_0354.add_value("ORL_O22", "ORL_O22", false);
        table_0354.add_value("ORL_O34", "ORL_O34", false);
        table_0354.add_value("ORL_O36", "ORL_O36", false);
        table_0354.add_value("ORL_O40", "ORL_O40", false);
        table_0354.add_value("ORL_O41", "ORL_O41", false);
        table_0354.add_value("ORL_O42", "ORL_O42", false);
        table_0354.add_value("ORL_O43", "ORL_O43", false);
        table_0354.add_value("ORL_O44", "ORL_O44", false);
        table_0354.add_value("ORM_O01", "ORM_O01", false);
        table_0354.add_value("ORM_Q06", "ORM_Q06", false);
        table_0354.add_value("ORN_O02", "ORN_O02", false);
        table_0354.add_value("ORN_O08", "ORN_O08", false);
        table_0354.add_value("ORP_O10", "ORP_O10", false);
        table_0354.add_value("ORR_O02", "ORR_O02", false);
        table_0354.add_value("ORR_Q06", "ORR_Q06", false);
        table_0354.add_value("ORS_O02", "ORS_O02", false);
        table_0354.add_value("ORS_O06", "ORS_O06", false);
        table_0354.add_value("ORU_R01", "ORU_R01", false);
        table_0354.add_value("ORU_R30", "ORU_R30", false);
        table_0354.add_value("ORU_R31", "ORU_R31", false);
        table_0354.add_value("ORU_R32", "ORU_R32", false);
        table_0354.add_value("ORU_W01", "ORU_W01", false);
        table_0354.add_value("ORX_O43", "ORX_O43", false);
        table_0354.add_value("OSM_R26", "OSM_R26", false);
        table_0354.add_value("OSQ_Q06", "OSQ_Q06", false);
        table_0354.add_value("OSR_Q06", "OSR_Q06", false);
        table_0354.add_value("OSU_O41", "OSU_O41", false);
        table_0354.add_value("OUL_R21", "OUL_R21", false);
        table_0354.add_value("OUL_R22", "OUL_R22", false);
        table_0354.add_value("OUL_R23", "OUL_R23", false);
        table_0354.add_value("OUL_R24", "OUL_R24", false);
        table_0354.add_value("PEX_P07", "PEX_P07", false);
        table_0354.add_value("PGL_PC6", "PGL_PC6", false);
        table_0354.add_value("PIN_I07", "PIN_I07", false);
        table_0354.add_value("PMU_B01", "PMU_B01", false);
        table_0354.add_value("PMU_B03", "PMU_B03", false);
        table_0354.add_value("PMU_B04", "PMU_B04", false);
        table_0354.add_value("PMU_B07", "PMU_B07", false);
        table_0354.add_value("PMU_B08", "PMU_B08", false);
        table_0354.add_value("PPG_PCG", "PPG_PCG", false);
        table_0354.add_value("PPP_PCB", "PPP_PCB", false);
        table_0354.add_value("PPR_PC1", "PPR_PC1", false);
        table_0354.add_value("PPT_PCL", "PPT_PCL", false);
        table_0354.add_value("PPV_PCA", "PPV_PCA", false);
        table_0354.add_value("PRR_PC5", "PRR_PC5", false);
        table_0354.add_value("PTR_PCF", "PTR_PCF", false);
        table_0354.add_value("QBP_E03", "QBP_E03", false);
        table_0354.add_value("QBP_E22", "QBP_E22", false);
        table_0354.add_value("QBP_O33", "QBP_O33", false);
        table_0354.add_value("QBP_O34", "QBP_O34", false);
        table_0354.add_value("QBP_Q11", "QBP_Q11", false);
        table_0354.add_value("QBP_Q13", "QBP_Q13", false);
        table_0354.add_value("QBP_Q15", "QBP_Q15", false);
        table_0354.add_value("QBP_Q21", "QBP_Q21", false);
        table_0354.add_value("QBP_Qnn", "QBP_Qnn", false);
        table_0354.add_value("QBP_Z73", "QBP_Z73", false);
        table_0354.add_value("QCK_Q02", "QCK_Q02", false);
        table_0354.add_value("QCN_J01", "QCN_J01", false);
        table_0354.add_value("QRF_W02", "QRF_W02", false);
        table_0354.add_value("QRY_A19", "QRY_A19", false);
        table_0354.add_value("QRY_P04", "QRY_P04", false);
        table_0354.add_value("QRY_PC4", "QRY_PC4", false);
        table_0354.add_value("QRY_Q01", "QRY_Q01", false);
        table_0354.add_value("QRY_Q02", "QRY_Q02", false);
        table_0354.add_value("QRY_R02", "QRY_R02", false);
        table_0354.add_value("QRY_T12", "QRY_T12", false);
        table_0354.add_value("QSB_Q16", "QSB_Q16", false);
        table_0354.add_value("QVR_Q17", "QVR_Q17", false);
        table_0354.add_value("RAR_RAR", "RAR_RAR", false);
        table_0354.add_value("RAS_O01", "RAS_O01", false);
        table_0354.add_value("RAS_O02", "RAS_O02", false);
        table_0354.add_value("RAS_O17", "RAS_O17", false);
        table_0354.add_value("RCI_I05", "RCI_I05", false);
        table_0354.add_value("RCL_I06", "RCL_I06", false);
        table_0354.add_value("RDE_O01", "RDE_O01", false);
        table_0354.add_value("RDE_O11", "RDE_O11", false);
        table_0354.add_value("RDO_O01", "RDO_O01", false);
        table_0354.add_value("RDR_RDR", "RDR_RDR", false);
        table_0354.add_value("RDS_O01", "RDS_O01", false);
        table_0354.add_value("RDS_O13", "RDS_O13", false);
        table_0354.add_value("RDY_K11", "RDY_K11", false);
        table_0354.add_value("RDY_K15", "RDY_K15", false);
        table_0354.add_value("REF_I12", "REF_I12", false);
        table_0354.add_value("RER_RER", "RER_RER", false);
        table_0354.add_value("RGR_RGR", "RGR_RGR", false);
        table_0354.add_value("RGV_O01", "RGV_O01", false);
        table_0354.add_value("RGV_O15", "RGV_O15", false);
        table_0354.add_value("ROR_ROR", "ROR_ROR", false);
        table_0354.add_value("RPA_I08", "RPA_I08", false);
        table_0354.add_value("RPI_I01", "RPI_I01", false);
        table_0354.add_value("RPI_I04", "RPI_I04", false);
        table_0354.add_value("RPI_I0I", "RPI_I0I", false);
        table_0354.add_value("RPL_I02", "RPL_I02", false);
        table_0354.add_value("RPR_I03", "RPR_I03", false);
        table_0354.add_value("RQA_I08", "RQA_I08", false);
        table_0354.add_value("RQC_I05", "RQC_I05", false);
        table_0354.add_value("RQC_I06", "RQC_I06", false);
        table_0354.add_value("RQI_I01", "RQI_I01", false);
        table_0354.add_value("RQI_I0I", "RQI_I0I", false);
        table_0354.add_value("RQP_I04", "RQP_I04", false);
        table_0354.add_value("RQQ_Q09", "RQQ_Q09", false);
        table_0354.add_value("RRA_O02", "RRA_O02", false);
        table_0354.add_value("RRA_O18", "RRA_O18", false);
        table_0354.add_value("RRD_O02", "RRD_O02", false);
        table_0354.add_value("RRD_O14", "RRD_O14", false);
        table_0354.add_value("RRE_O02", "RRE_O02", false);
        table_0354.add_value("RRE_O12", "RRE_O12", false);
        table_0354.add_value("RRG_O02", "RRG_O02", false);
        table_0354.add_value("RRG_O16", "RRG_O16", false);
        table_0354.add_value("RRI_I12", "RRI_I12", false);
        table_0354.add_value("RRO_O02", "RRO_O02", false);
        table_0354.add_value("RSP_E03", "RSP_E03", false);
        table_0354.add_value("RSP_E22", "RSP_E22", false);
        table_0354.add_value("RSP_K11", "RSP_K11", false);
        table_0354.add_value("RSP_K21", "RSP_K21", false);
        table_0354.add_value("RSP_K22", "RSP_K22", false);
        table_0354.add_value("RSP_K23", "RSP_K23", false);
        table_0354.add_value("RSP_K24", "RSP_K24", false);
        table_0354.add_value("RSP_K25", "RSP_K25", false);
        table_0354.add_value("RSP_K31", "RSP_K31", false);
        table_0354.add_value("RSP_K32", "RSP_K32", false);
        table_0354.add_value("RSP_O33", "RSP_O33", false);
        table_0354.add_value("RSP_O34", "RSP_O34", false);
        table_0354.add_value("RSP_Q11", "RSP_Q11", false);
        table_0354.add_value("RSP_Z82", "RSP_Z82", false);
        table_0354.add_value("RSP_Z86", "RSP_Z86", false);
        table_0354.add_value("RSP_Z88", "RSP_Z88", false);
        table_0354.add_value("RSP_Z90", "RSP_Z90", false);
        table_0354.add_value("RTB_K13", "RTB_K13", false);
        table_0354.add_value("RTB_Knn", "RTB_Knn", false);
        table_0354.add_value("RTB_Z74", "RTB_Z74", false);
        table_0354.add_value("SDR_S31", "SDR_S31", false);
        table_0354.add_value("SDR_S32", "SDR_S32", false);
        table_0354.add_value("SIU_S12", "SIU_S12", false);
        table_0354.add_value("SLR_S28", "SLR_S28", false);
        table_0354.add_value("SPQ_Q08", "SPQ_Q08", false);
        table_0354.add_value("SQM_S25", "SQM_S25", false);
        table_0354.add_value("SQR_S25", "SQR_S25", false);
        table_0354.add_value("SRM_S01", "SRM_S01", false);
        table_0354.add_value("SRM_T12", "SRM_T12", false);
        table_0354.add_value("SRR_S01", "SRR_S01", false);
        table_0354.add_value("SRR_T12", "SRR_T12", false);
        table_0354.add_value("SSR_U04", "SSR_U04", false);
        table_0354.add_value("SSU_U03", "SSU_U03", false);
        table_0354.add_value("STC_S33", "STC_S33", false);
        table_0354.add_value("SUR_P09", "SUR_P09", false);
        table_0354.add_value("TBR_R08", "TBR_R08", false);
        table_0354.add_value("TBR_R09", "TBR_R09", false);
        table_0354.add_value("TCU_U10", "TCU_U10", false);
        table_0354.add_value("UDM_Q05", "UDM_Q05", false);
        table_0354.add_value("VQQ_Q07", "VQQ_Q07", false);
        table_0354.add_value("VXQ_V01", "VXQ_V01", false);
        table_0354.add_value("VXR_V03", "VXR_V03", false);
        table_0354.add_value("VXU_V04", "VXU_V04", false);
        table_0354.add_value("VXX_V02", "VXX_V02", false);
        self.add_table(table_0354);

        // HL7 Table 0355 - Primary Key Value Type
        let mut table_0355 =
            Hl7Table::new("0355", "Primary Key Value Type", "Primary Key Value Type");
        table_0355.add_value("CE", "CE", false);
        table_0355.add_value("CWE", "CWE", false);
        table_0355.add_value("PL", "PL", false);
        table_0355.add_value("2.3", "2.3", false);
        table_0355.add_value("ISO 2022-1994", "ISO 2022-1994", false);
        table_0355.add_value("0", "0", false);
        table_0355.add_value("100", "100", false);
        table_0355.add_value("101", "101", false);
        table_0355.add_value("102", "102", false);
        table_0355.add_value("103", "103", false);
        table_0355.add_value("104", "104", false);
        table_0355.add_value("200", "200", false);
        table_0355.add_value("201", "201", false);
        table_0355.add_value("202", "202", false);
        table_0355.add_value("203", "203", false);
        table_0355.add_value("204", "204", false);
        table_0355.add_value("205", "205", false);
        table_0355.add_value("206", "206", false);
        table_0355.add_value("207", "207", false);
        table_0355.add_value("Errors", "Errors", false);
        table_0355.add_value("Rejection", "Rejection", false);
        table_0355.add_value("Success", "Success", false);
        self.add_table(table_0355);

        // HL7 Table 0356 - Alternate Character Set Handling Scheme
        let mut table_0356 = Hl7Table::new(
            "0356",
            "Alternate Character Set Handling Scheme",
            "Alternate Character Set Handling Scheme",
        );
        table_0356.add_value("2.3", "2.3", false);
        table_0356.add_value("ISO 2022-1994", "ISO 2022-1994", false);
        self.add_table(table_0356);

        // HL7 Table 0357 - Message Error Condition Codes
        let mut table_0357 = Hl7Table::new(
            "0357",
            "Message Error Condition Codes",
            "Message Error Condition Codes",
        );
        table_0357.add_value("0", "0", false);
        table_0357.add_value("100", "100", false);
        table_0357.add_value("101", "101", false);
        table_0357.add_value("102", "102", false);
        table_0357.add_value("103", "103", false);
        table_0357.add_value("104", "104", false);
        table_0357.add_value("200", "200", false);
        table_0357.add_value("201", "201", false);
        table_0357.add_value("202", "202", false);
        table_0357.add_value("203", "203", false);
        table_0357.add_value("204", "204", false);
        table_0357.add_value("205", "205", false);
        table_0357.add_value("206", "206", false);
        table_0357.add_value("207", "207", false);
        table_0357.add_value("Errors", "Errors", false);
        table_0357.add_value("Rejection", "Rejection", false);
        table_0357.add_value("Success", "Success", false);
        self.add_table(table_0357);

        // HL7 Table 0359 - Diagnosis Priority
        let mut table_0359 = Hl7Table::new("0359", "Diagnosis Priority", "Diagnosis Priority");
        table_0359.add_value("0", "0", false);
        table_0359.add_value("1", "1", false);
        table_0359.add_value("2", "2", false);
        table_0359.add_value("AA", "AA", false);
        table_0359.add_value("AAS", "AAS", false);
        table_0359.add_value("ABA", "ABA", false);
        table_0359.add_value("AE", "AE", false);
        table_0359.add_value("AS", "AS", false);
        table_0359.add_value("BA", "BA", false);
        table_0359.add_value("BBA", "BBA", false);
        table_0359.add_value("BE", "BE", false);
        table_0359.add_value("BFA", "BFA", false);
        table_0359.add_value("BN", "BN", false);
        table_0359.add_value("BS", "BS", false);
        table_0359.add_value("BSL", "BSL", false);
        table_0359.add_value("BSN", "BSN", false);
        table_0359.add_value("BT", "BT", false);
        table_0359.add_value("CANP", "CANP", false);
        table_0359.add_value("CER", "CER", false);
        table_0359.add_value("CMA", "CMA", false);
        table_0359.add_value("CNM", "CNM", false);
        table_0359.add_value("CNP", "CNP", false);
        table_0359.add_value("CNS", "CNS", false);
        table_0359.add_value("CPNP", "CPNP", false);
        table_0359.add_value("CRN", "CRN", false);
        table_0359.add_value("CTR", "CTR", false);
        table_0359.add_value("DBA", "DBA", false);
        table_0359.add_value("DED", "DED", false);
        table_0359.add_value("DIP", "DIP", false);
        table_0359.add_value("DO", "DO", false);
        table_0359.add_value("EMT", "EMT", false);
        table_0359.add_value("EMTP", "EMTP", false);
        table_0359.add_value("FPNP", "FPNP", false);
        table_0359.add_value("HS", "HS", false);
        table_0359.add_value("JD", "JD", false);
        table_0359.add_value("MA", "MA", false);
        table_0359.add_value("MBA", "MBA", false);
        table_0359.add_value("MCE", "MCE", false);
        table_0359.add_value("MD", "MD", false);
        table_0359.add_value("MDA", "MDA", false);
        table_0359.add_value("MDI", "MDI", false);
        table_0359.add_value("ME", "ME", false);
        table_0359.add_value("MED", "MED", false);
        table_0359.add_value("MEE", "MEE", false);
        table_0359.add_value("MFA", "MFA", false);
        table_0359.add_value("MME", "MME", false);
        table_0359.add_value("MS", "MS", false);
        table_0359.add_value("MSL", "MSL", false);
        table_0359.add_value("MSN", "MSN", false);
        table_0359.add_value("MT", "MT", false);
        table_0359.add_value("MTH", "MTH", false);
        table_0359.add_value("NG", "NG", false);
        table_0359.add_value("NP", "NP", false);
        table_0359.add_value("PA", "PA", false);
        table_0359.add_value("PHD", "PHD", false);
        table_0359.add_value("PHE", "PHE", false);
        table_0359.add_value("PHS", "PHS", false);
        table_0359.add_value("PN", "PN", false);
        table_0359.add_value("PharmD", "PharmD", false);
        table_0359.add_value("RMA", "RMA", false);
        table_0359.add_value("RN", "RN", false);
        table_0359.add_value("RPH", "RPH", false);
        table_0359.add_value("SEC", "SEC", false);
        table_0359.add_value("TS", "TS", false);
        table_0359.add_pattern(">=2");
        self.add_table(table_0359);

        // HL7 Table 0360 - Degree/License/Certificate
        let mut table_0360 = Hl7Table::new(
            "0360",
            "Degree/License/Certificate",
            "Degree/License/Certificate",
        );
        table_0360.add_value("AA", "AA", false);
        table_0360.add_value("AAS", "AAS", false);
        table_0360.add_value("ABA", "ABA", false);
        table_0360.add_value("AE", "AE", false);
        table_0360.add_value("AS", "AS", false);
        table_0360.add_value("BA", "BA", false);
        table_0360.add_value("BBA", "BBA", false);
        table_0360.add_value("BE", "BE", false);
        table_0360.add_value("BFA", "BFA", false);
        table_0360.add_value("BN", "BN", false);
        table_0360.add_value("BS", "BS", false);
        table_0360.add_value("BSL", "BSL", false);
        table_0360.add_value("BSN", "BSN", false);
        table_0360.add_value("BT", "BT", false);
        table_0360.add_value("CANP", "CANP", false);
        table_0360.add_value("CER", "CER", false);
        table_0360.add_value("CMA", "CMA", false);
        table_0360.add_value("CNM", "CNM", false);
        table_0360.add_value("CNP", "CNP", false);
        table_0360.add_value("CNS", "CNS", false);
        table_0360.add_value("CPNP", "CPNP", false);
        table_0360.add_value("CRN", "CRN", false);
        table_0360.add_value("CTR", "CTR", false);
        table_0360.add_value("DBA", "DBA", false);
        table_0360.add_value("DED", "DED", false);
        table_0360.add_value("DIP", "DIP", false);
        table_0360.add_value("DO", "DO", false);
        table_0360.add_value("EMT", "EMT", false);
        table_0360.add_value("EMTP", "EMTP", false);
        table_0360.add_value("FPNP", "FPNP", false);
        table_0360.add_value("HS", "HS", false);
        table_0360.add_value("JD", "JD", false);
        table_0360.add_value("MA", "MA", false);
        table_0360.add_value("MBA", "MBA", false);
        table_0360.add_value("MCE", "MCE", false);
        table_0360.add_value("MD", "MD", false);
        table_0360.add_value("MDA", "MDA", false);
        table_0360.add_value("MDI", "MDI", false);
        table_0360.add_value("ME", "ME", false);
        table_0360.add_value("MED", "MED", false);
        table_0360.add_value("MEE", "MEE", false);
        table_0360.add_value("MFA", "MFA", false);
        table_0360.add_value("MME", "MME", false);
        table_0360.add_value("MS", "MS", false);
        table_0360.add_value("MSL", "MSL", false);
        table_0360.add_value("MSN", "MSN", false);
        table_0360.add_value("MT", "MT", false);
        table_0360.add_value("MTH", "MTH", false);
        table_0360.add_value("NG", "NG", false);
        table_0360.add_value("NP", "NP", false);
        table_0360.add_value("PA", "PA", false);
        table_0360.add_value("PHD", "PHD", false);
        table_0360.add_value("PHE", "PHE", false);
        table_0360.add_value("PHS", "PHS", false);
        table_0360.add_value("PN", "PN", false);
        table_0360.add_value("PharmD", "PharmD", false);
        table_0360.add_value("RMA", "RMA", false);
        table_0360.add_value("RN", "RN", false);
        table_0360.add_value("RPH", "RPH", false);
        table_0360.add_value("SEC", "SEC", false);
        table_0360.add_value("TS", "TS", false);
        self.add_table(table_0360);

        // HL7 Table 0364 - Comment Type
        let mut table_0364 = Hl7Table::new("0364", "Comment Type", "Comment Type");
        table_0364.add_value("1R", "1R", false);
        table_0364.add_value("2R", "2R", false);
        table_0364.add_value("AI", "AI", false);
        table_0364.add_value("DR", "DR", false);
        table_0364.add_value("GI", "GI", false);
        table_0364.add_value("GR", "GR", false);
        table_0364.add_value("PI", "PI", false);
        table_0364.add_value("RE", "RE", false);
        self.add_table(table_0364);

        // HL7 Table 0369 - Specimen Role
        let mut table_0369 = Hl7Table::new("0369", "Specimen Role", "Specimen Role");
        table_0369.add_value("B", "B", false);
        table_0369.add_value("C", "C", false);
        table_0369.add_value("E", "E", false);
        table_0369.add_value("F", "F", false);
        table_0369.add_value("G", "G", false);
        table_0369.add_value("L", "L", false);
        table_0369.add_value("O", "O", false);
        table_0369.add_value("P", "P", false);
        table_0369.add_value("Q", "Q", false);
        table_0369.add_value("R", "R", false);
        table_0369.add_value("V", "V", false);
        self.add_table(table_0369);

        // HL7 Table 0370 - Container Status
        let mut table_0370 = Hl7Table::new("0370", "Container Status", "Container Status");
        table_0370.add_value("I", "I", false);
        table_0370.add_value("L", "L", false);
        table_0370.add_value("M", "M", false);
        table_0370.add_value("O", "O", false);
        table_0370.add_value("P", "P", false);
        table_0370.add_value("R", "R", false);
        table_0370.add_value("U", "U", false);
        table_0370.add_value("X", "X", false);
        self.add_table(table_0370);

        // HL7 Table 0371 - Additive/Preservative
        let mut table_0371 =
            Hl7Table::new("0371", "Additive/Preservative", "Additive/Preservative");
        table_0371.add_value("ACDA", "ACDA", false);
        table_0371.add_value("ACDB", "ACDB", false);
        table_0371.add_value("ACET", "ACET", false);
        table_0371.add_value("AMIES", "AMIES", false);
        table_0371.add_value("BACTM", "BACTM", false);
        table_0371.add_value("BF10", "BF10", false);
        table_0371.add_value("BOR", "BOR", false);
        table_0371.add_value("BOUIN", "BOUIN", false);
        table_0371.add_value("BSKM", "BSKM", false);
        table_0371.add_value("C32", "C32", false);
        table_0371.add_value("C38", "C38", false);
        table_0371.add_value("CARS", "CARS", false);
        table_0371.add_value("CARY", "CARY", false);
        table_0371.add_value("CHLTM", "CHLTM", false);
        table_0371.add_value("CTAD", "CTAD", false);
        table_0371.add_value("EDTK", "EDTK", false);
        table_0371.add_value("EDTK15", "EDTK15", false);
        table_0371.add_value("EDTK75", "EDTK75", false);
        table_0371.add_value("EDTN", "EDTN", false);
        table_0371.add_value("ENT", "ENT", false);
        table_0371.add_value("ENT+", "ENT+", false);
        table_0371.add_value("F10", "F10", false);
        table_0371.add_value("FDP", "FDP", false);
        table_0371.add_value("FL10", "FL10", false);
        table_0371.add_value("FL100", "FL100", false);
        table_0371.add_value("HCL6", "HCL6", false);
        table_0371.add_value("HEPA", "HEPA", false);
        table_0371.add_value("HEPL", "HEPL", false);
        table_0371.add_value("HEPN", "HEPN", false);
        table_0371.add_value("HNO3", "HNO3", false);
        table_0371.add_value("JKM", "JKM", false);
        table_0371.add_value("KARN", "KARN", false);
        table_0371.add_value("KOX", "KOX", false);
        table_0371.add_value("LIA", "LIA", false);
        table_0371.add_value("M4", "M4", false);
        table_0371.add_value("M4RT", "M4RT", false);
        table_0371.add_value("M5", "M5", false);
        table_0371.add_value("MICHTM", "MICHTM", false);
        table_0371.add_value("MMDTM", "MMDTM", false);
        table_0371.add_value("NAF", "NAF", false);
        table_0371.add_value("NAPS", "NAPS", false);
        table_0371.add_value("NONE", "NONE", false);
        table_0371.add_value("PAGE", "PAGE", false);
        table_0371.add_value("PHENOL", "PHENOL", false);
        table_0371.add_value("PVA", "PVA", false);
        table_0371.add_value("RLM", "RLM", false);
        table_0371.add_value("SILICA", "SILICA", false);
        table_0371.add_value("SPS", "SPS", false);
        table_0371.add_value("SST", "SST", false);
        table_0371.add_value("STUTM", "STUTM", false);
        table_0371.add_value("THROM", "THROM", false);
        table_0371.add_value("THYMOL", "THYMOL", false);
        table_0371.add_value("THYO", "THYO", false);
        table_0371.add_value("TOLU", "TOLU", false);
        table_0371.add_value("URETM", "URETM", false);
        table_0371.add_value("VIRTM", "VIRTM", false);
        table_0371.add_value("WEST", "WEST", false);
        self.add_table(table_0371);

        // HL7 Table 0372 - Specimen Component
        let mut table_0372 = Hl7Table::new("0372", "Specimen Component", "Specimen Component");
        table_0372.add_value("BLD", "BLD", false);
        table_0372.add_value("BSEP", "BSEP", false);
        table_0372.add_value("PLAS", "PLAS", false);
        table_0372.add_value("PPP", "PPP", false);
        table_0372.add_value("PRP", "PRP", false);
        table_0372.add_value("SED", "SED", false);
        table_0372.add_value("SER", "SER", false);
        table_0372.add_value("SUP", "SUP", false);
        self.add_table(table_0372);

        // HL7 Table 0373 - Treatment
        let mut table_0373 = Hl7Table::new("0373", "Treatment", "Treatment");
        table_0373.add_value("ACID", "ACID", false);
        table_0373.add_value("ALK", "ALK", false);
        table_0373.add_value("DEFB", "DEFB", false);
        table_0373.add_value("FILT", "FILT", false);
        table_0373.add_value("LDLP", "LDLP", false);
        table_0373.add_value("NEUT", "NEUT", false);
        table_0373.add_value("RECA", "RECA", false);
        table_0373.add_value("UFIL", "UFIL", false);
        self.add_table(table_0373);

        // HL7 Table 0374 - System Induced Contaminants
        let mut table_0374 = Hl7Table::new(
            "0374",
            "System Induced Contaminants",
            "System Induced Contaminants",
        );
        table_0374.add_value("CNTM", "CNTM", false);
        self.add_table(table_0374);

        // HL7 Table 0375 - Artificial Blood
        let mut table_0375 = Hl7Table::new("0375", "Artificial Blood", "Artificial Blood");
        table_0375.add_value("FLUR", "FLUR", false);
        table_0375.add_value("SFHB", "SFHB", false);
        self.add_table(table_0375);

        // HL7 Table 0376 - Special Handling Code
        let mut table_0376 =
            Hl7Table::new("0376", "Special Handling Code", "Special Handling Code");
        table_0376.add_value("AMB", "AMB", false);
        table_0376.add_value("C37", "C37", false);
        table_0376.add_value("CAMB", "CAMB", false);
        table_0376.add_value("CATM", "CATM", false);
        table_0376.add_value("CFRZ", "CFRZ", false);
        table_0376.add_value("CREF", "CREF", false);
        table_0376.add_value("DFRZ", "DFRZ", false);
        table_0376.add_value("DRY", "DRY", false);
        table_0376.add_value("FRZ", "FRZ", false);
        table_0376.add_value("MTLF", "MTLF", false);
        table_0376.add_value("NTR", "NTR", false);
        table_0376.add_value("PRTL", "PRTL", false);
        table_0376.add_value("PSA", "PSA", false);
        table_0376.add_value("PSO", "PSO", false);
        table_0376.add_value("REF", "REF", false);
        table_0376.add_value("UFRZ", "UFRZ", false);
        table_0376.add_value("UPR", "UPR", false);
        self.add_table(table_0376);

        // HL7 Table 0377 - Other Environmental Factors
        let mut table_0377 = Hl7Table::new(
            "0377",
            "Other Environmental Factors",
            "Other Environmental Factors",
        );
        table_0377.add_value("A60", "A60", false);
        table_0377.add_value("ATM", "ATM", false);
        self.add_table(table_0377);

        // HL7 Table 0389 - Analyte Repeat Status
        let mut table_0389 =
            Hl7Table::new("0389", "Analyte Repeat Status", "Analyte Repeat Status");
        table_0389.add_value("D", "D", false);
        table_0389.add_value("F", "F", false);
        table_0389.add_value("O", "O", false);
        table_0389.add_value("R", "R", false);
        self.add_table(table_0389);

        // HL7 Table 0396 - Coding System
        let mut table_0396 = Hl7Table::new("0396", "Coding System", "Coding System");
        table_0396.add_value("99IHE", "99IHE", false);
        table_0396.add_value("ACR", "ACR", false);
        table_0396.add_value("ALPHAID2006", "ALPHAID2006", false);
        table_0396.add_value("ALPHAID2007", "ALPHAID2007", false);
        table_0396.add_value("ALPHAID2008", "ALPHAID2008", false);
        table_0396.add_value("ALPHAID2009", "ALPHAID2009", false);
        table_0396.add_value("ALPHAID2010", "ALPHAID2010", false);
        table_0396.add_value("ALPHAID2011", "ALPHAID2011", false);
        table_0396.add_value("ANS+", "ANS+", false);
        table_0396.add_value("ART", "ART", false);
        table_0396.add_value("AS4", "AS4", false);
        table_0396.add_value("AS4E", "AS4E", false);
        table_0396.add_value("ATC", "ATC", false);
        table_0396.add_value("C4", "C4", false);
        table_0396.add_value("C5", "C5", false);
        table_0396.add_value("CAPECC", "CAPECC", false);
        table_0396.add_value("CAS", "CAS", false);
        table_0396.add_value("CCC", "CCC", false);
        table_0396.add_value("CD2", "CD2", false);
        table_0396.add_value("CDCA", "CDCA", false);
        table_0396.add_value("CDCEDACUITY", "CDCEDACUITY", false);
        table_0396.add_value("CDCM", "CDCM", false);
        table_0396.add_value("CDCOBS", "CDCOBS", false);
        table_0396.add_value("CDCPHINVS", "CDCPHINVS", false);
        table_0396.add_value("CDCREC", "CDCREC", false);
        table_0396.add_value("CDS", "CDS", false);
        table_0396.add_value("CE", "CE", false);
        table_0396.add_value("CLP", "CLP", false);
        table_0396.add_value("CPTM", "CPTM", false);
        table_0396.add_value("CST", "CST", false);
        table_0396.add_value("CVX", "CVX", false);
        table_0396.add_value("DCL", "DCL", false);
        table_0396.add_value("DCM", "DCM", false);
        table_0396.add_value("DQL", "DQL", false);
        table_0396.add_value("E", "E", false);
        table_0396.add_value("E5", "E5", false);
        table_0396.add_value("E6", "E6", false);
        table_0396.add_value("E7", "E7", false);
        table_0396.add_value("ENZC", "ENZC", false);
        table_0396.add_value("EPASRS", "EPASRS", false);
        table_0396.add_value("FDAUDI", "FDAUDI", false);
        table_0396.add_value("FDAUNII", "FDAUNII", false);
        table_0396.add_value("FDDC", "FDDC", false);
        table_0396.add_value("FDDX", "FDDX", false);
        table_0396.add_value("FDK", "FDK", false);
        table_0396.add_value("FIPS5_2", "FIPS5_2", false);
        table_0396.add_value("FIPS6_4", "FIPS6_4", false);
        table_0396.add_value("GDRG2004", "GDRG2004", false);
        table_0396.add_value("GDRG2005", "GDRG2005", false);
        table_0396.add_value("GDRG2006", "GDRG2006", false);
        table_0396.add_value("GDRG2007", "GDRG2007", false);
        table_0396.add_value("GDRG2008", "GDRG2008", false);
        table_0396.add_value("GDRG2009", "GDRG2009", false);
        table_0396.add_value("GMDC2004", "GMDC2004", false);
        table_0396.add_value("GMDC2005", "GMDC2005", false);
        table_0396.add_value("GMDC2006", "GMDC2006", false);
        table_0396.add_value("GMDC2007", "GMDC2007", false);
        table_0396.add_value("GMDC2008", "GMDC2008", false);
        table_0396.add_value("GMDC2009", "GMDC2009", false);
        table_0396.add_value("GS1UDI", "GS1UDI", false);
        table_0396.add_value("HB", "HB", false);
        table_0396.add_value("HCPCS", "HCPCS", false);
        table_0396.add_value("HCPT", "HCPT", false);
        table_0396.add_value("HHC", "HHC", false);
        table_0396.add_value("HI", "HI", false);
        table_0396.add_value("HIBUDI", "HIBUDI", false);
        table_0396.add_value("HOT", "HOT", false);
        table_0396.add_value("HPC", "HPC", false);
        table_0396.add_value("I10", "I10", false);
        table_0396.add_value("I10G2004", "I10G2004", false);
        table_0396.add_value("I10G2005", "I10G2005", false);
        table_0396.add_value("I10G2006", "I10G2006", false);
        table_0396.add_value("I10P", "I10P", false);
        table_0396.add_value("I9", "I9", false);
        table_0396.add_value("I9C", "I9C", false);
        table_0396.add_value("I9CDX", "I9CDX", false);
        table_0396.add_value("I9CP", "I9CP", false);
        table_0396.add_value("IBT", "IBT", false);
        table_0396.add_value("IC2", "IC2", false);
        table_0396.add_value("ICCUDI", "ICCUDI", false);
        table_0396.add_value("ICD10AM", "ICD10AM", false);
        table_0396.add_value("ICD10CA", "ICD10CA", false);
        table_0396.add_value("ICD10GM2007", "ICD10GM2007", false);
        table_0396.add_value("ICD10GM2008", "ICD10GM2008", false);
        table_0396.add_value("ICD10GM2009", "ICD10GM2009", false);
        table_0396.add_value("ICD10GM2010", "ICD10GM2010", false);
        table_0396.add_value("ICD10GM2011", "ICD10GM2011", false);
        table_0396.add_value("ICDO", "ICDO", false);
        table_0396.add_value("ICDO2", "ICDO2", false);
        table_0396.add_value("ICDO3", "ICDO3", false);
        table_0396.add_value("ICS", "ICS", false);
        table_0396.add_value("ICSD", "ICSD", false);
        table_0396.add_value("ISO", "ISO", false);
        table_0396.add_value("ISO+", "ISO+", false);
        table_0396.add_value("ISO3166_1", "ISO3166_1", false);
        table_0396.add_value("ISO3166_2", "ISO3166_2", false);
        table_0396.add_value("ISO4217", "ISO4217", false);
        table_0396.add_value("ISO639", "ISO639", false);
        table_0396.add_value("ITIS", "ITIS", false);
        table_0396.add_value("IUPC", "IUPC", false);
        table_0396.add_value("IUPP", "IUPP", false);
        table_0396.add_value("JC10", "JC10", false);
        table_0396.add_value("JC8", "JC8", false);
        table_0396.add_value("JJ1017", "JJ1017", false);
        table_0396.add_value("L", "L", false);
        table_0396.add_value("LB", "LB", false);
        table_0396.add_value("LN", "LN", false);
        table_0396.add_value("MCD", "MCD", false);
        table_0396.add_value("MCR", "MCR", false);
        table_0396.add_value("MDC", "MDC", false);
        table_0396.add_value("MDDX", "MDDX", false);
        table_0396.add_value("MEDC", "MEDC", false);
        table_0396.add_value("MEDR", "MEDR", false);
        table_0396.add_value("MEDX", "MEDX", false);
        table_0396.add_value("MGPI", "MGPI", false);
        table_0396.add_value("MVX", "MVX", false);
        table_0396.add_value("NABM", "NABM", false);
        table_0396.add_value("NAICS", "NAICS", false);
        table_0396.add_value("NDA", "NDA", false);
        table_0396.add_value("NDC", "NDC", false);
        table_0396.add_value("NDFRT", "NDFRT", false);
        table_0396.add_value("NIC", "NIC", false);
        table_0396.add_value("NIP001", "NIP001", false);
        table_0396.add_value("NIP002", "NIP002", false);
        table_0396.add_value("NIP004", "NIP004", false);
        table_0396.add_value("NIP007", "NIP007", false);
        table_0396.add_value("NIP008", "NIP008", false);
        table_0396.add_value("NIP009", "NIP009", false);
        table_0396.add_value("NIP010", "NIP010", false);
        table_0396.add_value("NND", "NND", false);
        table_0396.add_value("NPI", "NPI", false);
        table_0396.add_value("NUBC", "NUBC", false);
        table_0396.add_value("NULLFL", "NULLFL", false);
        table_0396.add_value("O301", "O301", false);
        table_0396.add_value("O3012004", "O3012004", false);
        table_0396.add_value("O3012005", "O3012005", false);
        table_0396.add_value("O3012006", "O3012006", false);
        table_0396.add_value("OHA", "OHA", false);
        table_0396.add_value("OPS2007", "OPS2007", false);
        table_0396.add_value("OPS2008", "OPS2008", false);
        table_0396.add_value("OPS2009", "OPS2009", false);
        table_0396.add_value("OPS2010", "OPS2010", false);
        table_0396.add_value("OPS2011", "OPS2011", false);
        table_0396.add_value("PHINQUESTION", "PHINQUESTION", false);
        table_0396.add_value("PLR", "PLR", false);
        table_0396.add_value("PLT", "PLT", false);
        table_0396.add_value("POS", "POS", false);
        table_0396.add_value("RC", "RC", false);
        table_0396.add_value("RXNORM", "RXNORM", false);
        table_0396.add_value("SCT", "SCT", false);
        table_0396.add_value("SCT2", "SCT2", false);
        table_0396.add_value("SDM", "SDM", false);
        table_0396.add_value("SIC", "SIC", false);
        table_0396.add_value("SNM", "SNM", false);
        table_0396.add_value("SNM3", "SNM3", false);
        table_0396.add_value("SNT", "SNT", false);
        table_0396.add_value("SOC", "SOC", false);
        table_0396.add_value("UB04FL14", "UB04FL14", false);
        table_0396.add_value("UB04FL15", "UB04FL15", false);
        table_0396.add_value("UB04FL17", "UB04FL17", false);
        table_0396.add_value("UB04FL31", "UB04FL31", false);
        table_0396.add_value("UB04FL35", "UB04FL35", false);
        table_0396.add_value("UB04FL39", "UB04FL39", false);
        table_0396.add_value("UC", "UC", false);
        table_0396.add_value("UCUM", "UCUM", false);
        table_0396.add_value("UMD", "UMD", false);
        table_0396.add_value("UML", "UML", false);
        table_0396.add_value("UPC", "UPC", false);
        table_0396.add_value("UPIN", "UPIN", false);
        table_0396.add_value("USPS", "USPS", false);
        table_0396.add_value("W1", "W1", false);
        table_0396.add_value("W2", "W2", false);
        table_0396.add_value("W4", "W4", false);
        table_0396.add_value("WC", "WC", false);
        table_0396.add_pattern("99zzz");
        table_0396.add_pattern("HL7nnnn");
        table_0396.add_pattern("IBTnnnn");
        table_0396.add_pattern("ISOnnnn");
        table_0396.add_pattern("NCPDPnnnnsss");
        table_0396.add_pattern("X12Dennnn");
        self.add_table(table_0396);

        // HL7 Table 0398 - Continuation Style Code
        let mut table_0398 =
            Hl7Table::new("0398", "Continuation Style Code", "Continuation Style Code");
        table_0398.add_value("F", "F", false);
        table_0398.add_value("I", "I", false);
        self.add_table(table_0398);

        // HL7 Table 0399 - Country Code
        let mut table_0399 = Hl7Table::new("0399", "Country Code", "Country Code");
        table_0399.add_value("ABW", "ABW", false);
        table_0399.add_value("AFG", "AFG", false);
        table_0399.add_value("AFT", "AFT", false);
        table_0399.add_value("AGO", "AGO", false);
        table_0399.add_value("AIA", "AIA", false);
        table_0399.add_value("ALB", "ALB", false);
        table_0399.add_value("AND", "AND", false);
        table_0399.add_value("ANT", "ANT", false);
        table_0399.add_value("ARE", "ARE", false);
        table_0399.add_value("ARG", "ARG", false);
        table_0399.add_value("ARM", "ARM", false);
        table_0399.add_value("ASM", "ASM", false);
        table_0399.add_value("ATA", "ATA", false);
        table_0399.add_value("ATG", "ATG", false);
        table_0399.add_value("AUS", "AUS", false);
        table_0399.add_value("AUT", "AUT", false);
        table_0399.add_value("AZE", "AZE", false);
        table_0399.add_value("BDI", "BDI", false);
        table_0399.add_value("BEL", "BEL", false);
        table_0399.add_value("BEN", "BEN", false);
        table_0399.add_value("BFA", "BFA", false);
        table_0399.add_value("BGD", "BGD", false);
        table_0399.add_value("BGR", "BGR", false);
        table_0399.add_value("BHR", "BHR", false);
        table_0399.add_value("BHS", "BHS", false);
        table_0399.add_value("BIH", "BIH", false);
        table_0399.add_value("BLR", "BLR", false);
        table_0399.add_value("BLZ", "BLZ", false);
        table_0399.add_value("BMU", "BMU", false);
        table_0399.add_value("BOL", "BOL", false);
        table_0399.add_value("BRA", "BRA", false);
        table_0399.add_value("BRB", "BRB", false);
        table_0399.add_value("BRN", "BRN", false);
        table_0399.add_value("BTN", "BTN", false);
        table_0399.add_value("BVT", "BVT", false);
        table_0399.add_value("BWA", "BWA", false);
        table_0399.add_value("CAF", "CAF", false);
        table_0399.add_value("CAN", "CAN", false);
        table_0399.add_value("CCK", "CCK", false);
        table_0399.add_value("CHE", "CHE", false);
        table_0399.add_value("CHL", "CHL", false);
        table_0399.add_value("CHN", "CHN", false);
        table_0399.add_value("CIV", "CIV", false);
        table_0399.add_value("CMR", "CMR", false);
        table_0399.add_value("COD", "COD", false);
        table_0399.add_value("COG", "COG", false);
        table_0399.add_value("COK", "COK", false);
        table_0399.add_value("COL", "COL", false);
        table_0399.add_value("COM", "COM", false);
        table_0399.add_value("CPV", "CPV", false);
        table_0399.add_value("CRI", "CRI", false);
        table_0399.add_value("CUB", "CUB", false);
        table_0399.add_value("CXR", "CXR", false);
        table_0399.add_value("CYM", "CYM", false);
        table_0399.add_value("CYP", "CYP", false);
        table_0399.add_value("CZE", "CZE", false);
        table_0399.add_value("DEU", "DEU", false);
        table_0399.add_value("DJI", "DJI", false);
        table_0399.add_value("DMA", "DMA", false);
        table_0399.add_value("DNK", "DNK", false);
        table_0399.add_value("DOM", "DOM", false);
        table_0399.add_value("DZA", "DZA", false);
        table_0399.add_value("ECU", "ECU", false);
        table_0399.add_value("EGY", "EGY", false);
        table_0399.add_value("ERI", "ERI", false);
        table_0399.add_value("ESH", "ESH", false);
        table_0399.add_value("ESP", "ESP", false);
        table_0399.add_value("EST", "EST", false);
        table_0399.add_value("ETH", "ETH", false);
        table_0399.add_value("FIN", "FIN", false);
        table_0399.add_value("FJI", "FJI", false);
        table_0399.add_value("FLK", "FLK", false);
        table_0399.add_value("FRA", "FRA", false);
        table_0399.add_value("FRO", "FRO", false);
        table_0399.add_value("FSM", "FSM", false);
        table_0399.add_value("GAB", "GAB", false);
        table_0399.add_value("GBR", "GBR", false);
        table_0399.add_value("GEO", "GEO", false);
        table_0399.add_value("GHA", "GHA", false);
        table_0399.add_value("GIB", "GIB", false);
        table_0399.add_value("GIN", "GIN", false);
        table_0399.add_value("GLP", "GLP", false);
        table_0399.add_value("GMB", "GMB", false);
        table_0399.add_value("GNB", "GNB", false);
        table_0399.add_value("GNQ", "GNQ", false);
        table_0399.add_value("GRC", "GRC", false);
        table_0399.add_value("GRD", "GRD", false);
        table_0399.add_value("GRL", "GRL", false);
        table_0399.add_value("GTM", "GTM", false);
        table_0399.add_value("GUF", "GUF", false);
        table_0399.add_value("GUM", "GUM", false);
        table_0399.add_value("GUY", "GUY", false);
        table_0399.add_value("HKG", "HKG", false);
        table_0399.add_value("HMD", "HMD", false);
        table_0399.add_value("HND", "HND", false);
        table_0399.add_value("HRV", "HRV", false);
        table_0399.add_value("HTI", "HTI", false);
        table_0399.add_value("HUN", "HUN", false);
        table_0399.add_value("IDN", "IDN", false);
        table_0399.add_value("IND", "IND", false);
        table_0399.add_value("IOT", "IOT", false);
        table_0399.add_value("IRL", "IRL", false);
        table_0399.add_value("IRN", "IRN", false);
        table_0399.add_value("IRQ", "IRQ", false);
        table_0399.add_value("ISL", "ISL", false);
        table_0399.add_value("ISR", "ISR", false);
        table_0399.add_value("ITA", "ITA", false);
        table_0399.add_value("JAM", "JAM", false);
        table_0399.add_value("JOR", "JOR", false);
        table_0399.add_value("JPN", "JPN", false);
        table_0399.add_value("KAZ", "KAZ", false);
        table_0399.add_value("KEN", "KEN", false);
        table_0399.add_value("KGZ", "KGZ", false);
        table_0399.add_value("KHM", "KHM", false);
        table_0399.add_value("KIR", "KIR", false);
        table_0399.add_value("KNA", "KNA", false);
        table_0399.add_value("KOR", "KOR", false);
        table_0399.add_value("KWT", "KWT", false);
        table_0399.add_value("LAO", "LAO", false);
        table_0399.add_value("LBN", "LBN", false);
        table_0399.add_value("LBR", "LBR", false);
        table_0399.add_value("LBY", "LBY", false);
        table_0399.add_value("LCA", "LCA", false);
        table_0399.add_value("LIE", "LIE", false);
        table_0399.add_value("LKA", "LKA", false);
        table_0399.add_value("LSO", "LSO", false);
        table_0399.add_value("LTU", "LTU", false);
        table_0399.add_value("LUX", "LUX", false);
        table_0399.add_value("LVA", "LVA", false);
        table_0399.add_value("MAC", "MAC", false);
        table_0399.add_value("MAR", "MAR", false);
        table_0399.add_value("MCO", "MCO", false);
        table_0399.add_value("MDA", "MDA", false);
        table_0399.add_value("MDG", "MDG", false);
        table_0399.add_value("MDV", "MDV", false);
        table_0399.add_value("MEX", "MEX", false);
        table_0399.add_value("MHL", "MHL", false);
        table_0399.add_value("MKD", "MKD", false);
        table_0399.add_value("MLI", "MLI", false);
        table_0399.add_value("MLT", "MLT", false);
        table_0399.add_value("MMR", "MMR", false);
        table_0399.add_value("MNG", "MNG", false);
        table_0399.add_value("MNP", "MNP", false);
        table_0399.add_value("MOZ", "MOZ", false);
        table_0399.add_value("MRT", "MRT", false);
        table_0399.add_value("MSR", "MSR", false);
        table_0399.add_value("MTQ", "MTQ", false);
        table_0399.add_value("MUS", "MUS", false);
        table_0399.add_value("MWI", "MWI", false);
        table_0399.add_value("MYS", "MYS", false);
        table_0399.add_value("MYT", "MYT", false);
        table_0399.add_value("NAM", "NAM", false);
        table_0399.add_value("NCL", "NCL", false);
        table_0399.add_value("NER", "NER", false);
        table_0399.add_value("NFK", "NFK", false);
        table_0399.add_value("NGA", "NGA", false);
        table_0399.add_value("NIC", "NIC", false);
        table_0399.add_value("NIU", "NIU", false);
        table_0399.add_value("NLD", "NLD", false);
        table_0399.add_value("NOR", "NOR", false);
        table_0399.add_value("NPL", "NPL", false);
        table_0399.add_value("NRU", "NRU", false);
        table_0399.add_value("NZL", "NZL", false);
        table_0399.add_value("OMN", "OMN", false);
        table_0399.add_value("PAK", "PAK", false);
        table_0399.add_value("PAN", "PAN", false);
        table_0399.add_value("PCN", "PCN", false);
        table_0399.add_value("PER", "PER", false);
        table_0399.add_value("PHL", "PHL", false);
        table_0399.add_value("PLW", "PLW", false);
        table_0399.add_value("PNG", "PNG", false);
        table_0399.add_value("POL", "POL", false);
        table_0399.add_value("PRI", "PRI", false);
        table_0399.add_value("PRK", "PRK", false);
        table_0399.add_value("PRT", "PRT", false);
        table_0399.add_value("PRY", "PRY", false);
        table_0399.add_value("PYF", "PYF", false);
        table_0399.add_value("QAT", "QAT", false);
        table_0399.add_value("REU", "REU", false);
        table_0399.add_value("ROM", "ROM", false);
        table_0399.add_value("RUS", "RUS", false);
        table_0399.add_value("RWA", "RWA", false);
        table_0399.add_value("SAU", "SAU", false);
        table_0399.add_value("SDN", "SDN", false);
        table_0399.add_value("SEN", "SEN", false);
        table_0399.add_value("SGP", "SGP", false);
        table_0399.add_value("SGS", "SGS", false);
        table_0399.add_value("SHN", "SHN", false);
        table_0399.add_value("SJM", "SJM", false);
        table_0399.add_value("SLB", "SLB", false);
        table_0399.add_value("SLE", "SLE", false);
        table_0399.add_value("SLV", "SLV", false);
        table_0399.add_value("SMR", "SMR", false);
        table_0399.add_value("SOM", "SOM", false);
        table_0399.add_value("SPM", "SPM", false);
        table_0399.add_value("STP", "STP", false);
        table_0399.add_value("SUR", "SUR", false);
        table_0399.add_value("SVK", "SVK", false);
        table_0399.add_value("SVN", "SVN", false);
        table_0399.add_value("SWE", "SWE", false);
        table_0399.add_value("SWZ", "SWZ", false);
        table_0399.add_value("SYC", "SYC", false);
        table_0399.add_value("SYR", "SYR", false);
        table_0399.add_value("TCA", "TCA", false);
        table_0399.add_value("TCD", "TCD", false);
        table_0399.add_value("TGO", "TGO", false);
        table_0399.add_value("THA", "THA", false);
        table_0399.add_value("TJK", "TJK", false);
        table_0399.add_value("TKL", "TKL", false);
        table_0399.add_value("TKM", "TKM", false);
        table_0399.add_value("TMP", "TMP", false);
        table_0399.add_value("TON", "TON", false);
        table_0399.add_value("TTO", "TTO", false);
        table_0399.add_value("TUN", "TUN", false);
        table_0399.add_value("TUR", "TUR", false);
        table_0399.add_value("TUV", "TUV", false);
        table_0399.add_value("TWN", "TWN", false);
        table_0399.add_value("TZA", "TZA", false);
        table_0399.add_value("UGA", "UGA", false);
        table_0399.add_value("UKR", "UKR", false);
        table_0399.add_value("UMI", "UMI", false);
        table_0399.add_value("URY", "URY", false);
        table_0399.add_value("USA", "USA", false);
        table_0399.add_value("UZB", "UZB", false);
        table_0399.add_value("VAT", "VAT", false);
        table_0399.add_value("VCT", "VCT", false);
        table_0399.add_value("VEN", "VEN", false);
        table_0399.add_value("VGB", "VGB", false);
        table_0399.add_value("VIR", "VIR", false);
        table_0399.add_value("VNM", "VNM", false);
        table_0399.add_value("VUT", "VUT", false);
        table_0399.add_value("WLF", "WLF", false);
        table_0399.add_value("WSM", "WSM", false);
        table_0399.add_value("YEM", "YEM", false);
        table_0399.add_value("YUG", "YUG", false);
        table_0399.add_value("ZAF", "ZAF", false);
        table_0399.add_value("ZMB", "ZMB", false);
        table_0399.add_value("ZWE", "ZWE", false);
        self.add_table(table_0399);

        // HL7 Table 0406 - Organization Unit Type
        let mut table_0406 =
            Hl7Table::new("0406", "Organization Unit Type", "Organization Unit Type");
        table_0406.add_value("1", "1", false);
        table_0406.add_value("2", "2", false);
        table_0406.add_value("3", "3", false);
        table_0406.add_value("4", "4", false);
        table_0406.add_value("5", "5", false);
        table_0406.add_value("H", "H", false);
        table_0406.add_value("O", "O", false);
        table_0406.add_value("1ST", "1ST", false);
        table_0406.add_value("2ND", "2ND", false);
        table_0406.add_value("3RD", "3RD", false);
        table_0406.add_value("4TH", "4TH", false);
        table_0406.add_value("5TH", "5TH", false);
        table_0406.add_value("A/P", "A/P", false);
        table_0406.add_value("ANT", "ANT", false);
        table_0406.add_value("BLT", "BLT", false);
        table_0406.add_value("DEC", "DEC", false);
        table_0406.add_value("DST", "DST", false);
        table_0406.add_value("LAT", "LAT", false);
        table_0406.add_value("LFT", "LFT", false);
        table_0406.add_value("LLQ", "LLQ", false);
        table_0406.add_value("LOW", "LOW", false);
        table_0406.add_value("LUQ", "LUQ", false);
        table_0406.add_value("MED", "MED", false);
        table_0406.add_value("OR", "OR", false);
        table_0406.add_value("PED", "PED", false);
        table_0406.add_value("POS", "POS", false);
        table_0406.add_value("PRT", "PRT", false);
        table_0406.add_value("PRX", "PRX", false);
        table_0406.add_value("REC", "REC", false);
        table_0406.add_value("RGH", "RGH", false);
        table_0406.add_value("RLQ", "RLQ", false);
        table_0406.add_value("RUQ", "RUQ", false);
        table_0406.add_value("UPP", "UPP", false);
        table_0406.add_value("UPR", "UPR", false);
        table_0406.add_value("WCT", "WCT", false);
        table_0406.add_value("WOC", "WOC", false);
        table_0406.add_value("WSD", "WSD", false);
        self.add_table(table_0406);

        // HL7 Table 0411 - Supplemental Service Information Values
        let mut table_0411 = Hl7Table::new(
            "0411",
            "Supplemental Service Information Values",
            "Supplemental Service Information Values",
        );
        table_0411.add_value("1ST", "1ST", false);
        table_0411.add_value("2ND", "2ND", false);
        table_0411.add_value("3RD", "3RD", false);
        table_0411.add_value("4TH", "4TH", false);
        table_0411.add_value("5TH", "5TH", false);
        table_0411.add_value("A/P", "A/P", false);
        table_0411.add_value("ANT", "ANT", false);
        table_0411.add_value("BLT", "BLT", false);
        table_0411.add_value("DEC", "DEC", false);
        table_0411.add_value("DST", "DST", false);
        table_0411.add_value("LAT", "LAT", false);
        table_0411.add_value("LFT", "LFT", false);
        table_0411.add_value("LLQ", "LLQ", false);
        table_0411.add_value("LOW", "LOW", false);
        table_0411.add_value("LUQ", "LUQ", false);
        table_0411.add_value("MED", "MED", false);
        table_0411.add_value("OR", "OR", false);
        table_0411.add_value("PED", "PED", false);
        table_0411.add_value("POS", "POS", false);
        table_0411.add_value("PRT", "PRT", false);
        table_0411.add_value("PRX", "PRX", false);
        table_0411.add_value("REC", "REC", false);
        table_0411.add_value("RGH", "RGH", false);
        table_0411.add_value("RLQ", "RLQ", false);
        table_0411.add_value("RUQ", "RUQ", false);
        table_0411.add_value("UPP", "UPP", false);
        table_0411.add_value("UPR", "UPR", false);
        table_0411.add_value("WCT", "WCT", false);
        table_0411.add_value("WOC", "WOC", false);
        table_0411.add_value("WSD", "WSD", false);
        self.add_table(table_0411);

        // HL7 Table 0415 - Transfer Type
        let mut table_0415 = Hl7Table::new("0415", "Transfer Type", "Transfer Type");
        table_0415.add_value("E", "E", false);
        table_0415.add_value("N", "N", false);
        self.add_table(table_0415);

        // HL7 Table 0416 - Procedure DRG Type
        let mut table_0416 = Hl7Table::new("0416", "Procedure DRG Type", "Procedure DRG Type");
        table_0416.add_value("1", "1", false);
        table_0416.add_value("2", "2", false);
        table_0416.add_value("3", "3", false);
        table_0416.add_value("4", "4", false);
        table_0416.add_value("5", "5", false);
        self.add_table(table_0416);

        // HL7 Table 0417 - Tissue Type Code
        let mut table_0417 = Hl7Table::new("0417", "Tissue Type Code", "Tissue Type Code");
        table_0417.add_value("0", "0", false);
        table_0417.add_value("1", "1", false);
        table_0417.add_value("2", "2", false);
        table_0417.add_value("3", "3", false);
        table_0417.add_value("4", "4", false);
        table_0417.add_value("5", "5", false);
        table_0417.add_value("6", "6", false);
        table_0417.add_value("7", "7", false);
        table_0417.add_value("8", "8", false);
        table_0417.add_value("9", "9", false);
        table_0417.add_value("B", "B", false);
        table_0417.add_value("C", "C", false);
        table_0417.add_value("G", "G", false);
        self.add_table(table_0417);

        // HL7 Table 0418 - Procedure Priority
        let mut table_0418 = Hl7Table::new("0418", "Procedure Priority", "Procedure Priority");
        table_0418.add_value("0", "0", false);
        table_0418.add_value("1", "1", false);
        table_0418.add_value("2", "2", false);
        table_0418.add_pattern(">=2");
        self.add_table(table_0418);

        // HL7 Table 0427 - Risk Management Incident Code
        let mut table_0427 = Hl7Table::new(
            "0427",
            "Risk Management Incident Code",
            "Risk Management Incident Code",
        );
        table_0427.add_value("B", "B", false);
        table_0427.add_value("C", "C", false);
        table_0427.add_value("D", "D", false);
        table_0427.add_value("E", "E", false);
        table_0427.add_value("F", "F", false);
        table_0427.add_value("H", "H", false);
        table_0427.add_value("I", "I", false);
        table_0427.add_value("J", "J", false);
        table_0427.add_value("K", "K", false);
        table_0427.add_value("O", "O", false);
        table_0427.add_value("P", "P", false);
        table_0427.add_value("R", "R", false);
        table_0427.add_value("S", "S", false);
        table_0427.add_value("T", "T", false);
        self.add_table(table_0427);

        // HL7 Table 0429 - Production Class Code
        let mut table_0429 =
            Hl7Table::new("0429", "Production Class Code", "Production Class Code");
        table_0429.add_value("BR", "BR", false);
        table_0429.add_value("DA", "DA", false);
        table_0429.add_value("DR", "DR", false);
        table_0429.add_value("DU", "DU", false);
        table_0429.add_value("LY", "LY", false);
        table_0429.add_value("MT", "MT", false);
        table_0429.add_value("NA", "NA", false);
        table_0429.add_value("OT", "OT", false);
        table_0429.add_value("PL", "PL", false);
        table_0429.add_value("RA", "RA", false);
        table_0429.add_value("SH", "SH", false);
        table_0429.add_value("U", "U", false);
        self.add_table(table_0429);

        // HL7 Table 0430 - Mode of Arrival Code
        let mut table_0430 = Hl7Table::new("0430", "Mode of Arrival Code", "Mode of Arrival Code");
        table_0430.add_value("A", "A", false);
        table_0430.add_value("C", "C", false);
        table_0430.add_value("F", "F", false);
        table_0430.add_value("H", "H", false);
        table_0430.add_value("O", "O", false);
        table_0430.add_value("P", "P", false);
        table_0430.add_value("U", "U", false);
        table_0430.add_value("K", "K", false);
        table_0430.add_value("M", "M", false);
        table_0430.add_value("T", "T", false);
        table_0430.add_value("AC", "AC", false);
        table_0430.add_value("CH", "CH", false);
        table_0430.add_value("CO", "CO", false);
        table_0430.add_value("CR", "CR", false);
        table_0430.add_value("IM", "IM", false);
        table_0430.add_value("MO", "MO", false);
        self.add_table(table_0430);

        // HL7 Table 0431 - Recreational Drug Use Code
        let mut table_0431 = Hl7Table::new(
            "0431",
            "Recreational Drug Use Code",
            "Recreational Drug Use Code",
        );
        table_0431.add_value("A", "A", false);
        table_0431.add_value("C", "C", false);
        table_0431.add_value("K", "K", false);
        table_0431.add_value("M", "M", false);
        table_0431.add_value("O", "O", false);
        table_0431.add_value("T", "T", false);
        table_0431.add_value("U", "U", false);
        self.add_table(table_0431);

        // HL7 Table 0432 - Admission Level of Care Code
        let mut table_0432 = Hl7Table::new(
            "0432",
            "Admission Level of Care Code",
            "Admission Level of Care Code",
        );
        table_0432.add_value("AC", "AC", false);
        table_0432.add_value("CH", "CH", false);
        table_0432.add_value("CO", "CO", false);
        table_0432.add_value("CR", "CR", false);
        table_0432.add_value("IM", "IM", false);
        table_0432.add_value("MO", "MO", false);
        self.add_table(table_0432);

        // HL7 Table 0433 - Precaution Code
        let mut table_0433 = Hl7Table::new("0433", "Precaution Code", "Precaution Code");
        table_0433.add_value("A", "A", false);
        table_0433.add_value("B", "B", false);
        table_0433.add_value("C", "C", false);
        table_0433.add_value("D", "D", false);
        table_0433.add_value("I", "I", false);
        table_0433.add_value("N", "N", false);
        table_0433.add_value("O", "O", false);
        table_0433.add_value("P", "P", false);
        table_0433.add_value("U", "U", false);
        self.add_table(table_0433);

        // HL7 Table 0434 - Patient Condition Code
        let mut table_0434 =
            Hl7Table::new("0434", "Patient Condition Code", "Patient Condition Code");
        table_0434.add_value("A", "A", false);
        table_0434.add_value("C", "C", false);
        table_0434.add_value("O", "O", false);
        table_0434.add_value("P", "P", false);
        table_0434.add_value("S", "S", false);
        table_0434.add_value("U", "U", false);
        self.add_table(table_0434);

        // HL7 Table 0435 - Advance Directive Code
        let mut table_0435 =
            Hl7Table::new("0435", "Advance Directive Code", "Advance Directive Code");
        table_0435.add_value("DNR", "DNR", false);
        table_0435.add_value("N", "N", false);
        table_0435.add_value("A", "A", false);
        table_0435.add_value("I", "I", false);
        table_0435.add_value("L", "L", false);
        table_0435.add_value("M", "M", false);
        table_0435.add_value("O", "O", false);
        table_0435.add_value("P", "P", false);
        table_0435.add_value("U", "U", false);
        self.add_table(table_0435);

        // HL7 Table 0441 - Immunization Registry Status
        let mut table_0441 = Hl7Table::new(
            "0441",
            "Immunization Registry Status",
            "Immunization Registry Status",
        );
        table_0441.add_value("A", "A", false);
        table_0441.add_value("I", "I", false);
        table_0441.add_value("L", "L", false);
        table_0441.add_value("M", "M", false);
        table_0441.add_value("O", "O", false);
        table_0441.add_value("P", "P", false);
        table_0441.add_value("U", "U", false);
        self.add_table(table_0441);

        // HL7 Table 0443 - Provider Role
        let mut table_0443 = Hl7Table::new("0443", "Provider Role", "Provider Role");
        table_0443.add_value("AD", "AD", false);
        table_0443.add_value("AI", "AI", false);
        table_0443.add_value("AP", "AP", false);
        table_0443.add_value("AT", "AT", false);
        table_0443.add_value("CLP", "CLP", false);
        table_0443.add_value("CP", "CP", false);
        table_0443.add_value("DP", "DP", false);
        table_0443.add_value("EP", "EP", false);
        table_0443.add_value("FHCP", "FHCP", false);
        table_0443.add_value("IP", "IP", false);
        table_0443.add_value("MDIR", "MDIR", false);
        table_0443.add_value("OP", "OP", false);
        table_0443.add_value("PH", "PH", false);
        table_0443.add_value("PI", "PI", false);
        table_0443.add_value("PP", "PP", false);
        table_0443.add_value("RO", "RO", false);
        table_0443.add_value("RP", "RP", false);
        table_0443.add_value("RT", "RT", false);
        table_0443.add_value("TN", "TN", false);
        table_0443.add_value("TR", "TR", false);
        table_0443.add_value("VP", "VP", false);
        table_0443.add_value("VPS", "VPS", false);
        table_0443.add_value("VTS", "VTS", false);
        self.add_table(table_0443);

        // HL7 Table 0444 - Name Assembly Order
        let mut table_0444 = Hl7Table::new("0444", "Name Assembly Order", "Name Assembly Order");
        table_0444.add_value("F", "F", false);
        table_0444.add_value("G", "G", false);
        table_0444.add_value("AL", "AL", false);
        table_0444.add_value("UA", "UA", false);
        table_0444.add_value("UD", "UD", false);
        table_0444.add_value("US", "US", false);
        self.add_table(table_0444);

        // HL7 Table 0445 - Identity Reliability Code
        let mut table_0445 = Hl7Table::new(
            "0445",
            "Identity Reliability Code",
            "Identity Reliability Code",
        );
        table_0445.add_value("AL", "AL", false);
        table_0445.add_value("UA", "UA", false);
        table_0445.add_value("UD", "UD", false);
        table_0445.add_value("US", "US", false);
        self.add_table(table_0445);

        // HL7 Table 0456 - Revenue code
        let mut table_0456 = Hl7Table::new("0456", "Revenue code", "Revenue code");
        table_0456.add_value("260", "260", false);
        table_0456.add_value("280", "280", false);
        table_0456.add_value("301", "301", false);
        table_0456.add_value("991", "991", false);
        table_0456.add_value("993", "993", false);
        table_0456.add_value("994", "994", false);
        self.add_table(table_0456);

        // HL7 Table 0465 - Name/Address Representation
        let mut table_0465 = Hl7Table::new(
            "0465",
            "Name/Address Representation",
            "Name/Address Representation",
        );
        table_0465.add_value("A", "A", false);
        table_0465.add_value("I", "I", false);
        table_0465.add_value("P", "P", false);
        self.add_table(table_0465);

        // HL7 Table 0472 - TQ Conjunction ID
        let mut table_0472 = Hl7Table::new("0472", "TQ Conjunction ID", "TQ Conjunction ID");
        table_0472.add_value("A", "A", false);
        table_0472.add_value("C", "C", false);
        table_0472.add_value("S", "S", false);
        self.add_table(table_0472);

        // HL7 Table 0475 - Charge Type Reason
        let mut table_0475 = Hl7Table::new("0475", "Charge Type Reason", "Charge Type Reason");
        table_0475.add_value("01", "01", false);
        table_0475.add_value("02", "02", false);
        table_0475.add_value("03", "03", false);
        table_0475.add_value("04", "04", false);
        table_0475.add_value("05", "05", false);
        table_0475.add_value("1", "1", false);
        table_0475.add_value("2", "2", false);
        table_0475.add_value("3", "3", false);
        table_0475.add_value("4", "4", false);
        table_0475.add_value("5", "5", false);
        table_0475.add_value("I", "I", false);
        table_0475.add_value("II", "II", false);
        table_0475.add_value("III", "III", false);
        table_0475.add_value("IV", "IV", false);
        table_0475.add_value("V", "V", false);
        table_0475.add_value("VI", "VI", false);
        self.add_table(table_0475);

        // HL7 Table 0477 - Controlled Substance Schedule*
        let mut table_0477 = Hl7Table::new(
            "0477",
            "Controlled Substance Schedule*",
            "Controlled Substance Schedule*",
        );
        table_0477.add_value("I", "I", false);
        table_0477.add_value("II", "II", false);
        table_0477.add_value("III", "III", false);
        table_0477.add_value("IV", "IV", false);
        table_0477.add_value("V", "V", false);
        table_0477.add_value("VI", "VI", false);
        self.add_table(table_0477);

        // HL7 Table 0478 - Formulary Status
        let mut table_0478 = Hl7Table::new("0478", "Formulary Status", "Formulary Status");
        table_0478.add_value("G", "G", false);
        table_0478.add_value("N", "N", false);
        table_0478.add_value("R", "R", false);
        table_0478.add_value("Y", "Y", false);
        self.add_table(table_0478);

        // HL7 Table 0480 - Pharmacy Order Types
        let mut table_0480 = Hl7Table::new("0480", "Pharmacy Order Types", "Pharmacy Order Types");
        table_0480.add_value("M", "M", false);
        table_0480.add_value("O", "O", false);
        table_0480.add_value("S", "S", false);
        self.add_table(table_0480);

        // HL7 Table 0482 - Order Type
        let mut table_0482 = Hl7Table::new("0482", "Order Type", "Order Type");
        table_0482.add_value("I", "I", false);
        table_0482.add_value("O", "O", false);
        self.add_table(table_0482);

        // HL7 Table 0483 - Authorization Mode
        let mut table_0483 = Hl7Table::new("0483", "Authorization Mode", "Authorization Mode");
        table_0483.add_value("EL", "EL", false);
        table_0483.add_value("EM", "EM", false);
        table_0483.add_value("FX", "FX", false);
        table_0483.add_value("IP", "IP", false);
        table_0483.add_value("MA", "MA", false);
        table_0483.add_value("PA", "PA", false);
        table_0483.add_value("PH", "PH", false);
        table_0483.add_value("RE", "RE", false);
        table_0483.add_value("VC", "VC", false);
        table_0483.add_value("VO", "VO", false);
        self.add_table(table_0483);

        // HL7 Table 0484 - Dispense Type
        let mut table_0484 = Hl7Table::new("0484", "Dispense Type", "Dispense Type");
        table_0484.add_value("B", "B", false);
        table_0484.add_value("C", "C", false);
        table_0484.add_value("N", "N", false);
        table_0484.add_value("P", "P", false);
        table_0484.add_value("Q", "Q", false);
        table_0484.add_value("R", "R", false);
        table_0484.add_value("S", "S", false);
        table_0484.add_value("T", "T", false);
        table_0484.add_value("Z", "Z", false);
        self.add_table(table_0484);

        // HL7 Table 0485 - Extended Priority Codes
        let mut table_0485 =
            Hl7Table::new("0485", "Extended Priority Codes", "Extended Priority Codes");
        table_0485.add_value("A", "A", false);
        table_0485.add_value("C", "C", false);
        table_0485.add_value("P", "P", false);
        table_0485.add_value("PRN", "PRN", false);
        table_0485.add_value("R", "R", false);
        table_0485.add_value("S", "S", false);
        table_0485.add_value("T", "T", false);
        table_0485.add_pattern("TD<integer>");
        table_0485.add_pattern("TH<integer>");
        table_0485.add_pattern("TL<integer>");
        table_0485.add_pattern("TM<integer>");
        table_0485.add_pattern("TS<integer>");
        table_0485.add_pattern("TW<integer>");
        self.add_table(table_0485);

        // HL7 Table 0487 - Specimen Type
        let mut table_0487 = Hl7Table::new("0487", "Specimen Type", "Specimen Type");
        table_0487.add_value("ABS", "ABS", false);
        table_0487.add_value("ACNE", "ACNE", false);
        table_0487.add_value("ACNFLD", "ACNFLD", false);
        table_0487.add_value("AIRS", "AIRS", false);
        table_0487.add_value("ALL", "ALL", false);
        table_0487.add_value("AMN", "AMN", false);
        table_0487.add_value("AMP", "AMP", false);
        table_0487.add_value("ANGI", "ANGI", false);
        table_0487.add_value("ARTC", "ARTC", false);
        table_0487.add_value("ASERU", "ASERU", false);
        table_0487.add_value("ASP", "ASP", false);
        table_0487.add_value("ATTE", "ATTE", false);
        table_0487.add_value("AUTOA", "AUTOA", false);
        table_0487.add_value("AUTOC", "AUTOC", false);
        table_0487.add_value("AUTP", "AUTP", false);
        table_0487.add_value("BBL", "BBL", false);
        table_0487.add_value("BCYST", "BCYST", false);
        table_0487.add_value("BDY", "BDY", false);
        table_0487.add_value("BIFL", "BIFL", false);
        table_0487.add_value("BITE", "BITE", false);
        table_0487.add_value("BLD", "BLD", false);
        table_0487.add_value("BLDA", "BLDA", false);
        table_0487.add_value("BLDC", "BLDC", false);
        table_0487.add_value("BLDCO", "BLDCO", false);
        table_0487.add_value("BLDV", "BLDV", false);
        table_0487.add_value("BLEB", "BLEB", false);
        table_0487.add_value("BLIST", "BLIST", false);
        table_0487.add_value("BOIL", "BOIL", false);
        table_0487.add_value("BON", "BON", false);
        table_0487.add_value("BOWL", "BOWL", false);
        table_0487.add_value("BPH", "BPH", false);
        table_0487.add_value("BPU", "BPU", false);
        table_0487.add_value("BRN", "BRN", false);
        table_0487.add_value("BRO", "BRO", false);
        table_0487.add_value("BRSH", "BRSH", false);
        table_0487.add_value("BRTH", "BRTH", false);
        table_0487.add_value("BRUS", "BRUS", false);
        table_0487.add_value("BUB", "BUB", false);
        table_0487.add_value("BULLA", "BULLA", false);
        table_0487.add_value("BX", "BX", false);
        table_0487.add_value("CALC", "CALC", false);
        table_0487.add_value("CARBU", "CARBU", false);
        table_0487.add_value("CAT", "CAT", false);
        table_0487.add_value("CBITE", "CBITE", false);
        table_0487.add_value("CDM", "CDM", false);
        table_0487.add_value("CLIPP", "CLIPP", false);
        table_0487.add_value("CNJT", "CNJT", false);
        table_0487.add_value("CNL", "CNL", false);
        table_0487.add_value("COL", "COL", false);
        table_0487.add_value("CONE", "CONE", false);
        table_0487.add_value("CSCR", "CSCR", false);
        table_0487.add_value("CSERU", "CSERU", false);
        table_0487.add_value("CSF", "CSF", false);
        table_0487.add_value("CSITE", "CSITE", false);
        table_0487.add_value("CSMY", "CSMY", false);
        table_0487.add_value("CST", "CST", false);
        table_0487.add_value("CSVR", "CSVR", false);
        table_0487.add_value("CTP", "CTP", false);
        table_0487.add_value("CUR", "CUR", false);
        table_0487.add_value("CVM", "CVM", false);
        table_0487.add_value("CVPS", "CVPS", false);
        table_0487.add_value("CVPT", "CVPT", false);
        table_0487.add_value("CVX", "CVX", false);
        table_0487.add_value("CYN", "CYN", false);
        table_0487.add_value("CYST", "CYST", false);
        table_0487.add_value("DBITE", "DBITE", false);
        table_0487.add_value("DCS", "DCS", false);
        table_0487.add_value("DEC", "DEC", false);
        table_0487.add_value("DEION", "DEION", false);
        table_0487.add_value("DIA", "DIA", false);
        table_0487.add_value("DIAF", "DIAF", false);
        table_0487.add_value("DISCHG", "DISCHG", false);
        table_0487.add_value("DIV", "DIV", false);
        table_0487.add_value("DOSE", "DOSE", false);
        table_0487.add_value("DRN", "DRN", false);
        table_0487.add_value("DRNG", "DRNG", false);
        table_0487.add_value("DRNGP", "DRNGP", false);
        table_0487.add_value("DUFL", "DUFL", false);
        table_0487.add_value("EAR", "EAR", false);
        table_0487.add_value("EARW", "EARW", false);
        table_0487.add_value("EBRUSH", "EBRUSH", false);
        table_0487.add_value("EEYE", "EEYE", false);
        table_0487.add_value("EFF", "EFF", false);
        table_0487.add_value("EFFUS", "EFFUS", false);
        table_0487.add_value("EFOD", "EFOD", false);
        table_0487.add_value("EISO", "EISO", false);
        table_0487.add_value("ELT", "ELT", false);
        table_0487.add_value("ENDC", "ENDC", false);
        table_0487.add_value("ENDM", "ENDM", false);
        table_0487.add_value("ENVIR", "ENVIR", false);
        table_0487.add_value("EOS", "EOS", false);
        table_0487.add_value("EOTH", "EOTH", false);
        table_0487.add_value("ESOI", "ESOI", false);
        table_0487.add_value("ESOS", "ESOS", false);
        table_0487.add_value("ETA", "ETA", false);
        table_0487.add_value("ETTP", "ETTP", false);
        table_0487.add_value("ETTUB", "ETTUB", false);
        table_0487.add_value("EWHI", "EWHI", false);
        table_0487.add_value("EXG", "EXG", false);
        table_0487.add_value("EXS", "EXS", false);
        table_0487.add_value("EXUDTE", "EXUDTE", false);
        table_0487.add_value("EYE", "EYE", false);
        table_0487.add_value("FAW", "FAW", false);
        table_0487.add_value("FBLOOD", "FBLOOD", false);
        table_0487.add_value("FGA", "FGA", false);
        table_0487.add_value("FIB", "FIB", false);
        table_0487.add_value("FIST", "FIST", false);
        table_0487.add_value("FLD", "FLD", false);
        table_0487.add_value("FLT", "FLT", false);
        table_0487.add_value("FLU", "FLU", false);
        table_0487.add_value("FLUID", "FLUID", false);
        table_0487.add_value("FOLEY", "FOLEY", false);
        table_0487.add_value("FRS", "FRS", false);
        table_0487.add_value("FSCLP", "FSCLP", false);
        table_0487.add_value("FUR", "FUR", false);
        table_0487.add_value("GAS", "GAS", false);
        table_0487.add_value("GASA", "GASA", false);
        table_0487.add_value("GASAN", "GASAN", false);
        table_0487.add_value("GASBR", "GASBR", false);
        table_0487.add_value("GASD", "GASD", false);
        table_0487.add_value("GAST", "GAST", false);
        table_0487.add_value("GEN", "GEN", false);
        table_0487.add_value("GENC", "GENC", false);
        table_0487.add_value("GENL", "GENL", false);
        table_0487.add_value("GENV", "GENV", false);
        table_0487.add_value("GRAFT", "GRAFT", false);
        table_0487.add_value("GRAFTS", "GRAFTS", false);
        table_0487.add_value("GRANU", "GRANU", false);
        table_0487.add_value("GROSH", "GROSH", false);
        table_0487.add_value("GSOL", "GSOL", false);
        table_0487.add_value("GSPEC", "GSPEC", false);
        table_0487.add_value("GT", "GT", false);
        table_0487.add_value("GTUBE", "GTUBE", false);
        table_0487.add_value("HAR", "HAR", false);
        table_0487.add_value("HBITE", "HBITE", false);
        table_0487.add_value("HBLUD", "HBLUD", false);
        table_0487.add_value("HEMAQ", "HEMAQ", false);
        table_0487.add_value("HEMO", "HEMO", false);
        table_0487.add_value("HERNI", "HERNI", false);
        table_0487.add_value("HEV", "HEV", false);
        table_0487.add_value("HIC", "HIC", false);
        table_0487.add_value("HYDC", "HYDC", false);
        table_0487.add_value("IBITE", "IBITE", false);
        table_0487.add_value("ICYST", "ICYST", false);
        table_0487.add_value("IDC", "IDC", false);
        table_0487.add_value("IHG", "IHG", false);
        table_0487.add_value("ILEO", "ILEO", false);
        table_0487.add_value("ILLEG", "ILLEG", false);
        table_0487.add_value("IMP", "IMP", false);
        table_0487.add_value("INCI", "INCI", false);
        table_0487.add_value("INFIL", "INFIL", false);
        table_0487.add_value("INS", "INS", false);
        table_0487.add_value("INTRD", "INTRD", false);
        table_0487.add_value("ISLT", "ISLT", false);
        table_0487.add_value("IT", "IT", false);
        table_0487.add_value("IUD", "IUD", false);
        table_0487.add_value("IVCAT", "IVCAT", false);
        table_0487.add_value("IVFLD", "IVFLD", false);
        table_0487.add_value("IVTIP", "IVTIP", false);
        table_0487.add_value("JEJU", "JEJU", false);
        table_0487.add_value("JNTFLD", "JNTFLD", false);
        table_0487.add_value("JP", "JP", false);
        table_0487.add_value("KELOI", "KELOI", false);
        table_0487.add_value("KIDFLD", "KIDFLD", false);
        table_0487.add_value("LAM", "LAM", false);
        table_0487.add_value("LAVG", "LAVG", false);
        table_0487.add_value("LAVGG", "LAVGG", false);
        table_0487.add_value("LAVGP", "LAVGP", false);
        table_0487.add_value("LAVPG", "LAVPG", false);
        table_0487.add_value("LENS1", "LENS1", false);
        table_0487.add_value("LENS2", "LENS2", false);
        table_0487.add_value("LESN", "LESN", false);
        table_0487.add_value("LIQ", "LIQ", false);
        table_0487.add_value("LIQO", "LIQO", false);
        table_0487.add_value("LN", "LN", false);
        table_0487.add_value("LNA", "LNA", false);
        table_0487.add_value("LNV", "LNV", false);
        table_0487.add_value("LSAC", "LSAC", false);
        table_0487.add_value("LYM", "LYM", false);
        table_0487.add_value("MAC", "MAC", false);
        table_0487.add_value("MAHUR", "MAHUR", false);
        table_0487.add_value("MAR", "MAR", false);
        table_0487.add_value("MASS", "MASS", false);
        table_0487.add_value("MBLD", "MBLD", false);
        table_0487.add_value("MEC", "MEC", false);
        table_0487.add_value("MILK", "MILK", false);
        table_0487.add_value("MLK", "MLK", false);
        table_0487.add_value("MUCOS", "MUCOS", false);
        table_0487.add_value("MUCUS", "MUCUS", false);
        table_0487.add_value("NAIL", "NAIL", false);
        table_0487.add_value("NASDR", "NASDR", false);
        table_0487.add_value("NEDL", "NEDL", false);
        table_0487.add_value("NEPH", "NEPH", false);
        table_0487.add_value("NGASP", "NGASP", false);
        table_0487.add_value("NGAST", "NGAST", false);
        table_0487.add_value("NGS", "NGS", false);
        table_0487.add_value("NODUL", "NODUL", false);
        table_0487.add_value("NOS", "NOS", false);
        table_0487.add_value("NSECR", "NSECR", false);
        table_0487.add_value("ORH", "ORH", false);
        table_0487.add_value("ORL", "ORL", false);
        table_0487.add_value("OTH", "OTH", false);
        table_0487.add_value("PACEM", "PACEM", false);
        table_0487.add_value("PAFL", "PAFL", false);
        table_0487.add_value("PCFL", "PCFL", false);
        table_0487.add_value("PDSIT", "PDSIT", false);
        table_0487.add_value("PDTS", "PDTS", false);
        table_0487.add_value("PELVA", "PELVA", false);
        table_0487.add_value("PENIL", "PENIL", false);
        table_0487.add_value("PERIA", "PERIA", false);
        table_0487.add_value("PILOC", "PILOC", false);
        table_0487.add_value("PINS", "PINS", false);
        table_0487.add_value("PIS", "PIS", false);
        table_0487.add_value("PLAN", "PLAN", false);
        table_0487.add_value("PLAS", "PLAS", false);
        table_0487.add_value("PLB", "PLB", false);
        table_0487.add_value("PLC", "PLC", false);
        table_0487.add_value("PLEVS", "PLEVS", false);
        table_0487.add_value("PLR", "PLR", false);
        table_0487.add_value("PMN", "PMN", false);
        table_0487.add_value("PND", "PND", false);
        table_0487.add_value("POL", "POL", false);
        table_0487.add_value("POPGS", "POPGS", false);
        table_0487.add_value("POPLG", "POPLG", false);
        table_0487.add_value("POPLV", "POPLV", false);
        table_0487.add_value("PORTA", "PORTA", false);
        table_0487.add_value("PPP", "PPP", false);
        table_0487.add_value("PROST", "PROST", false);
        table_0487.add_value("PRP", "PRP", false);
        table_0487.add_value("PRT", "PRT", false);
        table_0487.add_value("PSC", "PSC", false);
        table_0487.add_value("PUNCT", "PUNCT", false);
        table_0487.add_value("PUS", "PUS", false);
        table_0487.add_value("PUSFR", "PUSFR", false);
        table_0487.add_value("PUST", "PUST", false);
        table_0487.add_value("QC3", "QC3", false);
        table_0487.add_value("RANDU", "RANDU", false);
        table_0487.add_value("RBC", "RBC", false);
        table_0487.add_value("RBITE", "RBITE", false);
        table_0487.add_value("RECT", "RECT", false);
        table_0487.add_value("RECTA", "RECTA", false);
        table_0487.add_value("RENALC", "RENALC", false);
        table_0487.add_value("RENC", "RENC", false);
        table_0487.add_value("RES", "RES", false);
        table_0487.add_value("RT", "RT", false);
        table_0487.add_value("SAL", "SAL", false);
        table_0487.add_value("SCAR", "SCAR", false);
        table_0487.add_value("SCLV", "SCLV", false);
        table_0487.add_value("SCROA", "SCROA", false);
        table_0487.add_value("SECRE", "SECRE", false);
        table_0487.add_value("SER", "SER", false);
        table_0487.add_value("SHU", "SHU", false);
        table_0487.add_value("SHUNF", "SHUNF", false);
        table_0487.add_value("SHUNT", "SHUNT", false);
        table_0487.add_value("SITE", "SITE", false);
        table_0487.add_value("SKBP", "SKBP", false);
        table_0487.add_value("SKM", "SKM", false);
        table_0487.add_value("SKN", "SKN", false);
        table_0487.add_value("SMM", "SMM", false);
        table_0487.add_value("SMN", "SMN", false);
        table_0487.add_value("SNV", "SNV", false);
        table_0487.add_value("SPRM", "SPRM", false);
        table_0487.add_value("SPRP", "SPRP", false);
        table_0487.add_value("SPRPB", "SPRPB", false);
        table_0487.add_value("SPS", "SPS", false);
        table_0487.add_value("SPT", "SPT", false);
        table_0487.add_value("SPTC", "SPTC", false);
        table_0487.add_value("SPTT", "SPTT", false);
        table_0487.add_value("SPUT1", "SPUT1", false);
        table_0487.add_value("SPUTIN", "SPUTIN", false);
        table_0487.add_value("SPUTSP", "SPUTSP", false);
        table_0487.add_value("STER", "STER", false);
        table_0487.add_value("STL", "STL", false);
        table_0487.add_value("STONE", "STONE", false);
        table_0487.add_value("SUBMA", "SUBMA", false);
        table_0487.add_value("SUBMX", "SUBMX", false);
        table_0487.add_value("SUMP", "SUMP", false);
        table_0487.add_value("SUP", "SUP", false);
        table_0487.add_value("SUTUR", "SUTUR", false);
        table_0487.add_value("SWGZ", "SWGZ", false);
        table_0487.add_value("SWT", "SWT", false);
        table_0487.add_value("TASP", "TASP", false);
        table_0487.add_value("TEAR", "TEAR", false);
        table_0487.add_value("THRB", "THRB", false);
        table_0487.add_value("THRT", "THRT", false);
        table_0487.add_value("TISG", "TISG", false);
        table_0487.add_value("TISPL", "TISPL", false);
        table_0487.add_value("TISS", "TISS", false);
        table_0487.add_value("TISU", "TISU", false);
        table_0487.add_value("TLC", "TLC", false);
        table_0487.add_value("TLGI", "TLGI", false);
        table_0487.add_value("TLNG", "TLNG", false);
        table_0487.add_value("TRAC", "TRAC", false);
        table_0487.add_value("TRANS", "TRANS", false);
        table_0487.add_value("TSERU", "TSERU", false);
        table_0487.add_value("TSMI", "TSMI", false);
        table_0487.add_value("TSTES", "TSTES", false);
        table_0487.add_value("TTRA", "TTRA", false);
        table_0487.add_value("TUB", "TUB", false);
        table_0487.add_value("TUBES", "TUBES", false);
        table_0487.add_value("TUMOR", "TUMOR", false);
        table_0487.add_value("TZANC", "TZANC", false);
        table_0487.add_value("UDENT", "UDENT", false);
        table_0487.add_value("ULC", "ULC", false);
        table_0487.add_value("UMB", "UMB", false);
        table_0487.add_value("UMED", "UMED", false);
        table_0487.add_value("UR", "UR", false);
        table_0487.add_value("URC", "URC", false);
        table_0487.add_value("URINB", "URINB", false);
        table_0487.add_value("URINC", "URINC", false);
        table_0487.add_value("URINM", "URINM", false);
        table_0487.add_value("URINN", "URINN", false);
        table_0487.add_value("URINP", "URINP", false);
        table_0487.add_value("URNS", "URNS", false);
        table_0487.add_value("URT", "URT", false);
        table_0487.add_value("URTH", "URTH", false);
        table_0487.add_value("USCOP", "USCOP", false);
        table_0487.add_value("USPEC", "USPEC", false);
        table_0487.add_value("USUB", "USUB", false);
        table_0487.add_value("VASTIP", "VASTIP", false);
        table_0487.add_value("VENT", "VENT", false);
        table_0487.add_value("VITF", "VITF", false);
        table_0487.add_value("VOM", "VOM", false);
        table_0487.add_value("WASH", "WASH", false);
        table_0487.add_value("WASI", "WASI", false);
        table_0487.add_value("WAT", "WAT", false);
        table_0487.add_value("WB", "WB", false);
        table_0487.add_value("WBC", "WBC", false);
        table_0487.add_value("WEN", "WEN", false);
        table_0487.add_value("WICK", "WICK", false);
        table_0487.add_value("WND", "WND", false);
        table_0487.add_value("WNDA", "WNDA", false);
        table_0487.add_value("WNDD", "WNDD", false);
        table_0487.add_value("WNDE", "WNDE", false);
        table_0487.add_value("WORM", "WORM", false);
        table_0487.add_value("WRT", "WRT", false);
        table_0487.add_value("WWA", "WWA", false);
        table_0487.add_value("WWO", "WWO", false);
        table_0487.add_value("WWT", "WWT", false);
        table_0487.add_value("ANP", "ANP", false);
        table_0487.add_value("BAP", "BAP", false);
        table_0487.add_value("BCAE", "BCAE", false);
        table_0487.add_value("BCAN", "BCAN", false);
        table_0487.add_value("BCPD", "BCPD", false);
        table_0487.add_value("BIO", "BIO", false);
        table_0487.add_value("CAP", "CAP", false);
        table_0487.add_value("CATH", "CATH", false);
        table_0487.add_value("CVP", "CVP", false);
        table_0487.add_value("EPLA", "EPLA", false);
        table_0487.add_value("ESWA", "ESWA", false);
        table_0487.add_value("FNA", "FNA", false);
        table_0487.add_value("KOFFP", "KOFFP", false);
        table_0487.add_value("MARTL", "MARTL", false);
        table_0487.add_value("ML11", "ML11", false);
        table_0487.add_value("MLP", "MLP", false);
        table_0487.add_value("NYP", "NYP", false);
        table_0487.add_value("PACE", "PACE", false);
        table_0487.add_value("PIN", "PIN", false);
        table_0487.add_value("PNA", "PNA", false);
        table_0487.add_value("PRIME", "PRIME", false);
        table_0487.add_value("PUMP", "PUMP", false);
        table_0487.add_value("QC5", "QC5", false);
        table_0487.add_value("SCLP", "SCLP", false);
        table_0487.add_value("SCRAPS", "SCRAPS", false);
        table_0487.add_value("SHA", "SHA", false);
        table_0487.add_value("SWA", "SWA", false);
        table_0487.add_value("SWD", "SWD", false);
        table_0487.add_value("TMAN", "TMAN", false);
        table_0487.add_value("TMCH", "TMCH", false);
        table_0487.add_value("TMM4", "TMM4", false);
        table_0487.add_value("TMMY", "TMMY", false);
        table_0487.add_value("TMOT", "TMOT", false);
        table_0487.add_value("TMP", "TMP", false);
        table_0487.add_value("TMPV", "TMPV", false);
        table_0487.add_value("TMSC", "TMSC", false);
        table_0487.add_value("TMUP", "TMUP", false);
        table_0487.add_value("TMVI", "TMVI", false);
        table_0487.add_value("VENIP", "VENIP", false);
        table_0487.add_value("WOOD", "WOOD", false);
        self.add_table(table_0487);

        // HL7 Table 0488 - Specimen Collection Method
        let mut table_0488 = Hl7Table::new(
            "0488",
            "Specimen Collection Method",
            "Specimen Collection Method",
        );
        table_0488.add_value("ANP", "ANP", false);
        table_0488.add_value("BAP", "BAP", false);
        table_0488.add_value("BCAE", "BCAE", false);
        table_0488.add_value("BCAN", "BCAN", false);
        table_0488.add_value("BCPD", "BCPD", false);
        table_0488.add_value("BIO", "BIO", false);
        table_0488.add_value("CAP", "CAP", false);
        table_0488.add_value("CATH", "CATH", false);
        table_0488.add_value("CVP", "CVP", false);
        table_0488.add_value("EPLA", "EPLA", false);
        table_0488.add_value("ESWA", "ESWA", false);
        table_0488.add_value("FNA", "FNA", false);
        table_0488.add_value("KOFFP", "KOFFP", false);
        table_0488.add_value("LNA", "LNA", false);
        table_0488.add_value("LNV", "LNV", false);
        table_0488.add_value("MARTL", "MARTL", false);
        table_0488.add_value("ML11", "ML11", false);
        table_0488.add_value("MLP", "MLP", false);
        table_0488.add_value("NYP", "NYP", false);
        table_0488.add_value("PACE", "PACE", false);
        table_0488.add_value("PIN", "PIN", false);
        table_0488.add_value("PNA", "PNA", false);
        table_0488.add_value("PRIME", "PRIME", false);
        table_0488.add_value("PUMP", "PUMP", false);
        table_0488.add_value("QC5", "QC5", false);
        table_0488.add_value("SCLP", "SCLP", false);
        table_0488.add_value("SCRAPS", "SCRAPS", false);
        table_0488.add_value("SHA", "SHA", false);
        table_0488.add_value("SWA", "SWA", false);
        table_0488.add_value("SWD", "SWD", false);
        table_0488.add_value("TMAN", "TMAN", false);
        table_0488.add_value("TMCH", "TMCH", false);
        table_0488.add_value("TMM4", "TMM4", false);
        table_0488.add_value("TMMY", "TMMY", false);
        table_0488.add_value("TMOT", "TMOT", false);
        table_0488.add_value("TMP", "TMP", false);
        table_0488.add_value("TMPV", "TMPV", false);
        table_0488.add_value("TMSC", "TMSC", false);
        table_0488.add_value("TMUP", "TMUP", false);
        table_0488.add_value("TMVI", "TMVI", false);
        table_0488.add_value("VENIP", "VENIP", false);
        table_0488.add_value("WOOD", "WOOD", false);
        self.add_table(table_0488);

        // HL7 Table 0489 - Risk Codes
        let mut table_0489 = Hl7Table::new("0489", "Risk Codes", "Risk Codes");
        table_0489.add_value("AGG", "AGG", false);
        table_0489.add_value("BHZ", "BHZ", false);
        table_0489.add_value("BIO", "BIO", false);
        table_0489.add_value("COR", "COR", false);
        table_0489.add_value("ESC", "ESC", false);
        table_0489.add_value("EXP", "EXP", false);
        table_0489.add_value("IFL", "IFL", false);
        table_0489.add_value("INF", "INF", false);
        table_0489.add_value("INJ", "INJ", false);
        table_0489.add_value("POI", "POI", false);
        table_0489.add_value("RAD", "RAD", false);
        self.add_table(table_0489);

        // HL7 Table 0490 - Specimen Reject Reason
        let mut table_0490 =
            Hl7Table::new("0490", "Specimen Reject Reason", "Specimen Reject Reason");
        table_0490.add_value("EX", "EX", false);
        table_0490.add_value("QS", "QS", false);
        table_0490.add_value("RA", "RA", false);
        table_0490.add_value("RB", "RB", false);
        table_0490.add_value("RC", "RC", false);
        table_0490.add_value("RD", "RD", false);
        table_0490.add_value("RE", "RE", false);
        table_0490.add_value("RH", "RH", false);
        table_0490.add_value("RI", "RI", false);
        table_0490.add_value("RM", "RM", false);
        table_0490.add_value("RN", "RN", false);
        table_0490.add_value("RP", "RP", false);
        table_0490.add_value("RR", "RR", false);
        table_0490.add_value("RS", "RS", false);
        self.add_table(table_0490);

        // HL7 Table 0491 - Specimen Quality
        let mut table_0491 = Hl7Table::new("0491", "Specimen Quality", "Specimen Quality");
        table_0491.add_value("E", "E", false);
        table_0491.add_value("F", "F", false);
        table_0491.add_value("G", "G", false);
        table_0491.add_value("P", "P", false);
        table_0491.add_value("??", "??", false);
        table_0491.add_value("A", "A", false);
        table_0491.add_value("I", "I", false);
        self.add_table(table_0491);

        // HL7 Table 0492 - Specimen Appropriateness
        let mut table_0492 = Hl7Table::new(
            "0492",
            "Specimen Appropriateness",
            "Specimen Appropriateness",
        );
        table_0492.add_value("??", "??", false);
        table_0492.add_value("A", "A", false);
        table_0492.add_value("I", "I", false);
        table_0492.add_value("P", "P", false);
        self.add_table(table_0492);

        // HL7 Table 0493 - Specimen Condition
        let mut table_0493 = Hl7Table::new("0493", "Specimen Condition", "Specimen Condition");
        table_0493.add_value("AUT", "AUT", false);
        table_0493.add_value("CFU", "CFU", false);
        table_0493.add_value("CLOT", "CLOT", false);
        table_0493.add_value("CON", "CON", false);
        table_0493.add_value("COOL", "COOL", false);
        table_0493.add_value("FROZ", "FROZ", false);
        table_0493.add_value("HEM", "HEM", false);
        table_0493.add_value("LIVE", "LIVE", false);
        table_0493.add_value("ROOM", "ROOM", false);
        table_0493.add_value("SNR", "SNR", false);
        self.add_table(table_0493);

        // HL7 Table 0494 - Specimen Child Role
        let mut table_0494 = Hl7Table::new("0494", "Specimen Child Role", "Specimen Child Role");
        table_0494.add_value("A", "A", false);
        table_0494.add_value("C", "C", false);
        table_0494.add_value("M", "M", false);
        self.add_table(table_0494);

        // HL7 Table 0495 - Body Site Modifier
        let mut table_0495 = Hl7Table::new("0495", "Body Site Modifier", "Body Site Modifier");
        table_0495.add_value("ANT", "ANT", false);
        table_0495.add_value("BIL", "BIL", false);
        table_0495.add_value("DIS", "DIS", false);
        table_0495.add_value("EXT", "EXT", false);
        table_0495.add_value("L", "L", false);
        table_0495.add_value("LAT", "LAT", false);
        table_0495.add_value("LLQ", "LLQ", false);
        table_0495.add_value("LOW", "LOW", false);
        table_0495.add_value("LUQ", "LUQ", false);
        table_0495.add_value("MED", "MED", false);
        table_0495.add_value("POS", "POS", false);
        table_0495.add_value("PRO", "PRO", false);
        table_0495.add_value("R", "R", false);
        table_0495.add_value("RLQ", "RLQ", false);
        table_0495.add_value("RUQ", "RUQ", false);
        table_0495.add_value("UPP", "UPP", false);
        self.add_table(table_0495);

        // HL7 Table 0496 - Consent Type
        let mut table_0496 = Hl7Table::new("0496", "Consent Type", "Consent Type");
        table_0496.add_value("001", "001", false);
        table_0496.add_value("002", "002", false);
        table_0496.add_value("003", "003", false);
        table_0496.add_value("004", "004", false);
        table_0496.add_value("005", "005", false);
        table_0496.add_value("006", "006", false);
        table_0496.add_value("007", "007", false);
        table_0496.add_value("008", "008", false);
        table_0496.add_value("009", "009", false);
        table_0496.add_value("010", "010", false);
        table_0496.add_value("011", "011", false);
        table_0496.add_value("012", "012", false);
        table_0496.add_value("013", "013", false);
        table_0496.add_value("014", "014", false);
        table_0496.add_value("015", "015", false);
        table_0496.add_value("016", "016", false);
        table_0496.add_value("017", "017", false);
        table_0496.add_value("018", "018", false);
        table_0496.add_value("019", "019", false);
        table_0496.add_value("020", "020", false);
        table_0496.add_value("021", "021", false);
        table_0496.add_value("022", "022", false);
        table_0496.add_value("023", "023", false);
        table_0496.add_value("024", "024", false);
        table_0496.add_value("025", "025", false);
        table_0496.add_value("026", "026", false);
        table_0496.add_value("027", "027", false);
        table_0496.add_value("028", "028", false);
        table_0496.add_value("029", "029", false);
        table_0496.add_value("030", "030", false);
        table_0496.add_value("031", "031", false);
        table_0496.add_value("032", "032", false);
        table_0496.add_value("033", "033", false);
        table_0496.add_value("034", "034", false);
        table_0496.add_value("035", "035", false);
        table_0496.add_value("036", "036", false);
        table_0496.add_value("037", "037", false);
        table_0496.add_value("038", "038", false);
        table_0496.add_value("039", "039", false);
        table_0496.add_value("040", "040", false);
        table_0496.add_value("041", "041", false);
        table_0496.add_value("042", "042", false);
        table_0496.add_value("043", "043", false);
        table_0496.add_value("044", "044", false);
        table_0496.add_value("045", "045", false);
        table_0496.add_value("046", "046", false);
        table_0496.add_value("047", "047", false);
        table_0496.add_value("048", "048", false);
        table_0496.add_value("049", "049", false);
        table_0496.add_value("050", "050", false);
        table_0496.add_value("051", "051", false);
        table_0496.add_value("052", "052", false);
        table_0496.add_value("053", "053", false);
        table_0496.add_value("054", "054", false);
        table_0496.add_value("055", "055", false);
        table_0496.add_value("056", "056", false);
        table_0496.add_value("057", "057", false);
        table_0496.add_value("058", "058", false);
        table_0496.add_value("059", "059", false);
        table_0496.add_value("060", "060", false);
        table_0496.add_value("061", "061", false);
        table_0496.add_value("062", "062", false);
        table_0496.add_value("063", "063", false);
        table_0496.add_value("064", "064", false);
        table_0496.add_value("065", "065", false);
        table_0496.add_value("066", "066", false);
        table_0496.add_value("067", "067", false);
        table_0496.add_value("068", "068", false);
        table_0496.add_value("069", "069", false);
        table_0496.add_value("070", "070", false);
        table_0496.add_value("071", "071", false);
        table_0496.add_value("072", "072", false);
        table_0496.add_value("073", "073", false);
        table_0496.add_value("074", "074", false);
        table_0496.add_value("075", "075", false);
        table_0496.add_value("076", "076", false);
        table_0496.add_value("077", "077", false);
        table_0496.add_value("078", "078", false);
        table_0496.add_value("079", "079", false);
        table_0496.add_value("080", "080", false);
        table_0496.add_value("081", "081", false);
        table_0496.add_value("082", "082", false);
        table_0496.add_value("083", "083", false);
        table_0496.add_value("084", "084", false);
        table_0496.add_value("085", "085", false);
        table_0496.add_value("086", "086", false);
        table_0496.add_value("087", "087", false);
        table_0496.add_value("088", "088", false);
        table_0496.add_value("089", "089", false);
        table_0496.add_value("090", "090", false);
        table_0496.add_value("091", "091", false);
        table_0496.add_value("092", "092", false);
        table_0496.add_value("093", "093", false);
        table_0496.add_value("094", "094", false);
        table_0496.add_value("095", "095", false);
        table_0496.add_value("096", "096", false);
        table_0496.add_value("097", "097", false);
        table_0496.add_value("098", "098", false);
        table_0496.add_value("099", "099", false);
        table_0496.add_value("1", "1", false);
        table_0496.add_value("10", "10", false);
        table_0496.add_value("100", "100", false);
        table_0496.add_value("101", "101", false);
        table_0496.add_value("102", "102", false);
        table_0496.add_value("103", "103", false);
        table_0496.add_value("104", "104", false);
        table_0496.add_value("105", "105", false);
        table_0496.add_value("106", "106", false);
        table_0496.add_value("107", "107", false);
        table_0496.add_value("108", "108", false);
        table_0496.add_value("109", "109", false);
        table_0496.add_value("11", "11", false);
        table_0496.add_value("110", "110", false);
        table_0496.add_value("111", "111", false);
        table_0496.add_value("112", "112", false);
        table_0496.add_value("113", "113", false);
        table_0496.add_value("1137", "1137", false);
        table_0496.add_value("114", "114", false);
        table_0496.add_value("115", "115", false);
        table_0496.add_value("116", "116", false);
        table_0496.add_value("117", "117", false);
        table_0496.add_value("118", "118", false);
        table_0496.add_value("119", "119", false);
        table_0496.add_value("12", "12", false);
        table_0496.add_value("120", "120", false);
        table_0496.add_value("121", "121", false);
        table_0496.add_value("122", "122", false);
        table_0496.add_value("123", "123", false);
        table_0496.add_value("124", "124", false);
        table_0496.add_value("125", "125", false);
        table_0496.add_value("126", "126", false);
        table_0496.add_value("127", "127", false);
        table_0496.add_value("128", "128", false);
        table_0496.add_value("129", "129", false);
        table_0496.add_value("13", "13", false);
        table_0496.add_value("130", "130", false);
        table_0496.add_value("131", "131", false);
        table_0496.add_value("132", "132", false);
        table_0496.add_value("133", "133", false);
        table_0496.add_value("134", "134", false);
        table_0496.add_value("135", "135", false);
        table_0496.add_value("136", "136", false);
        table_0496.add_value("14", "14", false);
        table_0496.add_value("15", "15", false);
        table_0496.add_value("16", "16", false);
        table_0496.add_value("17", "17", false);
        table_0496.add_value("18", "18", false);
        table_0496.add_value("19", "19", false);
        table_0496.add_value("2", "2", false);
        table_0496.add_value("20", "20", false);
        table_0496.add_value("21", "21", false);
        table_0496.add_value("22", "22", false);
        table_0496.add_value("23", "23", false);
        table_0496.add_value("24", "24", false);
        table_0496.add_value("25", "25", false);
        table_0496.add_value("26", "26", false);
        table_0496.add_value("27", "27", false);
        table_0496.add_value("28", "28", false);
        table_0496.add_value("29", "29", false);
        table_0496.add_value("3", "3", false);
        table_0496.add_value("30", "30", false);
        table_0496.add_value("31", "31", false);
        table_0496.add_value("32", "32", false);
        table_0496.add_value("33", "33", false);
        table_0496.add_value("34", "34", false);
        table_0496.add_value("35", "35", false);
        table_0496.add_value("36", "36", false);
        table_0496.add_value("37", "37", false);
        table_0496.add_value("38", "38", false);
        table_0496.add_value("39", "39", false);
        table_0496.add_value("4", "4", false);
        table_0496.add_value("40", "40", false);
        table_0496.add_value("41", "41", false);
        table_0496.add_value("42", "42", false);
        table_0496.add_value("43", "43", false);
        table_0496.add_value("44", "44", false);
        table_0496.add_value("45", "45", false);
        table_0496.add_value("46", "46", false);
        table_0496.add_value("47", "47", false);
        table_0496.add_value("48", "48", false);
        table_0496.add_value("49", "49", false);
        table_0496.add_value("5", "5", false);
        table_0496.add_value("50", "50", false);
        table_0496.add_value("51", "51", false);
        table_0496.add_value("52", "52", false);
        table_0496.add_value("53", "53", false);
        table_0496.add_value("54", "54", false);
        table_0496.add_value("55", "55", false);
        table_0496.add_value("56", "56", false);
        table_0496.add_value("57", "57", false);
        table_0496.add_value("58", "58", false);
        table_0496.add_value("59", "59", false);
        table_0496.add_value("6", "6", false);
        table_0496.add_value("60", "60", false);
        table_0496.add_value("61", "61", false);
        table_0496.add_value("62", "62", false);
        table_0496.add_value("63", "63", false);
        table_0496.add_value("64", "64", false);
        table_0496.add_value("65", "65", false);
        table_0496.add_value("66", "66", false);
        table_0496.add_value("67", "67", false);
        table_0496.add_value("68", "68", false);
        table_0496.add_value("69", "69", false);
        table_0496.add_value("7", "7", false);
        table_0496.add_value("70", "70", false);
        table_0496.add_value("71", "71", false);
        table_0496.add_value("72", "72", false);
        table_0496.add_value("73", "73", false);
        table_0496.add_value("74", "74", false);
        table_0496.add_value("75", "75", false);
        table_0496.add_value("76", "76", false);
        table_0496.add_value("77", "77", false);
        table_0496.add_value("78", "78", false);
        table_0496.add_value("79", "79", false);
        table_0496.add_value("8", "8", false);
        table_0496.add_value("80", "80", false);
        table_0496.add_value("81", "81", false);
        table_0496.add_value("82", "82", false);
        table_0496.add_value("83", "83", false);
        table_0496.add_value("84", "84", false);
        table_0496.add_value("85", "85", false);
        table_0496.add_value("86", "86", false);
        table_0496.add_value("87", "87", false);
        table_0496.add_value("88", "88", false);
        table_0496.add_value("89", "89", false);
        table_0496.add_value("9", "9", false);
        table_0496.add_value("90", "90", false);
        table_0496.add_value("91", "91", false);
        table_0496.add_value("92", "92", false);
        table_0496.add_value("93", "93", false);
        table_0496.add_value("94", "94", false);
        table_0496.add_value("95", "95", false);
        table_0496.add_value("96", "96", false);
        table_0496.add_value("97", "97", false);
        table_0496.add_value("98", "98", false);
        table_0496.add_value("99", "99", false);
        self.add_table(table_0496);

        // HL7 Table 0497 - Consent Mode
        let mut table_0497 = Hl7Table::new("0497", "Consent Mode", "Consent Mode");
        table_0497.add_value("T", "T", false);
        table_0497.add_value("V", "V", false);
        table_0497.add_value("W", "W", false);
        self.add_table(table_0497);

        // HL7 Table 0498 - Consent Status
        let mut table_0498 = Hl7Table::new("0498", "Consent Status", "Consent Status");
        table_0498.add_value("A", "A", false);
        table_0498.add_value("B", "B", false);
        table_0498.add_value("L", "L", false);
        table_0498.add_value("P", "P", false);
        table_0498.add_value("R", "R", false);
        table_0498.add_value("X", "X", false);
        self.add_table(table_0498);

        // HL7 Table 0499 - Consent Bypass Reason
        let mut table_0499 =
            Hl7Table::new("0499", "Consent Bypass Reason", "Consent Bypass Reason");
        table_0499.add_value("E", "E", false);
        table_0499.add_value("PJ", "PJ", false);
        self.add_table(table_0499);

        // HL7 Table 0500 - Consent Disclosure Level
        let mut table_0500 = Hl7Table::new(
            "0500",
            "Consent Disclosure Level",
            "Consent Disclosure Level",
        );
        table_0500.add_value("F", "F", false);
        table_0500.add_value("N", "N", false);
        table_0500.add_value("P", "P", false);
        self.add_table(table_0500);

        // HL7 Table 0501 - Consent Non-Disclosure Reason
        let mut table_0501 = Hl7Table::new(
            "0501",
            "Consent Non-Disclosure Reason",
            "Consent Non-Disclosure Reason",
        );
        table_0501.add_value("E", "E", false);
        table_0501.add_value("PR", "PR", false);
        table_0501.add_value("RX", "RX", false);
        self.add_table(table_0501);

        // HL7 Table 0502 - Non-Subject Consenter Reason
        let mut table_0502 = Hl7Table::new(
            "0502",
            "Non-Subject Consenter Reason",
            "Non-Subject Consenter Reason",
        );
        table_0502.add_value("LM", "LM", false);
        table_0502.add_value("MIN", "MIN", false);
        table_0502.add_value("NC", "NC", false);
        self.add_table(table_0502);

        // HL7 Table 0503 - Sequence/Results Flag
        let mut table_0503 =
            Hl7Table::new("0503", "Sequence/Results Flag", "Sequence/Results Flag");
        table_0503.add_value("C", "C", false);
        table_0503.add_value("R", "R", false);
        table_0503.add_value("S", "S", false);
        self.add_table(table_0503);

        // HL7 Table 0504 - Sequence Condition Code
        let mut table_0504 =
            Hl7Table::new("0504", "Sequence Condition Code", "Sequence Condition Code");
        table_0504.add_value("EE", "EE", false);
        table_0504.add_value("ES", "ES", false);
        table_0504.add_value("SE", "SE", false);
        table_0504.add_value("SS", "SS", false);
        table_0504.add_value("#", "#", false);
        table_0504.add_value("*", "*", false);
        table_0504.add_value("C", "C", false);
        table_0504.add_value("E", "E", false);
        table_0504.add_value("N", "N", false);
        table_0504.add_value("S", "S", false);
        table_0504.add_value("T", "T", false);
        table_0504.add_value("A", "A", false);
        table_0504.add_value("F", "F", false);
        self.add_table(table_0504);

        // HL7 Table 0505 - Cyclic Entry/Exit Indicator
        let mut table_0505 = Hl7Table::new(
            "0505",
            "Cyclic Entry/Exit Indicator",
            "Cyclic Entry/Exit Indicator",
        );
        table_0505.add_value("#", "#", false);
        table_0505.add_value("*", "*", false);
        self.add_table(table_0505);

        // HL7 Table 0506 - Service Request Relationship
        let mut table_0506 = Hl7Table::new(
            "0506",
            "Service Request Relationship",
            "Service Request Relationship",
        );
        table_0506.add_value("C", "C", false);
        table_0506.add_value("E", "E", false);
        table_0506.add_value("N", "N", false);
        table_0506.add_value("S", "S", false);
        table_0506.add_value("T", "T", false);
        self.add_table(table_0506);

        // HL7 Table 0507 - Observation Result Handling
        let mut table_0507 = Hl7Table::new(
            "0507",
            "Observation Result Handling",
            "Observation Result Handling",
        );
        table_0507.add_value("A", "A", false);
        table_0507.add_value("F", "F", false);
        table_0507.add_value("N", "N", false);
        self.add_table(table_0507);

        // HL7 Table 0516 - Error Severity
        let mut table_0516 = Hl7Table::new("0516", "Error Severity", "Error Severity");
        table_0516.add_value("E", "E", false);
        table_0516.add_value("F", "F", false);
        table_0516.add_value("I", "I", false);
        table_0516.add_value("W", "W", false);
        self.add_table(table_0516);

        // HL7 Table 0517 - Inform Person Code
        let mut table_0517 = Hl7Table::new("0517", "Inform Person Code", "Inform Person Code");
        table_0517.add_value("HD", "HD", false);
        table_0517.add_value("NPAT", "NPAT", false);
        table_0517.add_value("PAT", "PAT", false);
        table_0517.add_value("USR", "USR", false);
        self.add_table(table_0517);

        // HL7 Table 0518 - Override Type
        let mut table_0518 = Hl7Table::new("0518", "Override Type", "Override Type");
        table_0518.add_value("EQV", "EQV", false);
        table_0518.add_value("EXTN", "EXTN", false);
        table_0518.add_value("INLV", "INLV", false);
        self.add_table(table_0518);

        // HL7 Table 0520 - Message Waiting Priority
        let mut table_0520 = Hl7Table::new(
            "0520",
            "Message Waiting Priority",
            "Message Waiting Priority",
        );
        table_0520.add_value("H", "H", false);
        table_0520.add_value("L", "L", false);
        table_0520.add_value("M", "M", false);
        self.add_table(table_0520);

        // HL7 Table 0527 - Calendar Alignment
        let mut table_0527 = Hl7Table::new("0527", "Calendar Alignment", "Calendar Alignment");
        table_0527.add_value("DM", "DM", false);
        table_0527.add_value("DW", "DW", false);
        table_0527.add_value("DY", "DY", false);
        table_0527.add_value("HD", "HD", false);
        table_0527.add_value("MY", "MY", false);
        table_0527.add_value("NH", "NH", false);
        table_0527.add_value("SN", "SN", false);
        table_0527.add_value("WY", "WY", false);
        self.add_table(table_0527);

        // HL7 Table 0528 - Event Related Period
        let mut table_0528 = Hl7Table::new("0528", "Event Related Period", "Event Related Period");
        table_0528.add_value("AC", "AC", false);
        table_0528.add_value("ACD", "ACD", false);
        table_0528.add_value("ACM", "ACM", false);
        table_0528.add_value("ACV", "ACV", false);
        table_0528.add_value("HS", "HS", false);
        table_0528.add_value("IC", "IC", false);
        table_0528.add_value("ICD", "ICD", false);
        table_0528.add_value("ICM", "ICM", false);
        table_0528.add_value("ICV", "ICV", false);
        table_0528.add_value("PC", "PC", false);
        table_0528.add_value("PCD", "PCD", false);
        table_0528.add_value("PCM", "PCM", false);
        table_0528.add_value("PCV", "PCV", false);
        self.add_table(table_0528);

        // HL7 Table 0529 - Precision
        let mut table_0529 = Hl7Table::new("0529", "Precision", "Precision");
        table_0529.add_value("D", "D", false);
        table_0529.add_value("H", "H", false);
        table_0529.add_value("L", "L", false);
        table_0529.add_value("M", "M", false);
        table_0529.add_value("S", "S", false);
        table_0529.add_value("Y", "Y", false);
        self.add_table(table_0529);

        // HL7 Table 0534 - Notify Clergy Code
        let mut table_0534 = Hl7Table::new("0534", "Notify Clergy Code", "Notify Clergy Code");
        table_0534.add_value("L", "L", false);
        table_0534.add_value("N", "N", false);
        table_0534.add_value("O", "O", false);
        table_0534.add_value("U", "U", false);
        table_0534.add_value("Y", "Y", false);
        self.add_table(table_0534);

        // HL7 Table 0535 - Signature Code
        let mut table_0535 = Hl7Table::new("0535", "Signature Code", "Signature Code");
        table_0535.add_value("C", "C", false);
        table_0535.add_value("M", "M", false);
        table_0535.add_value("P", "P", false);
        table_0535.add_value("S", "S", false);
        self.add_table(table_0535);

        // HL7 Table 0544 - Container Condition
        let mut table_0544 = Hl7Table::new("0544", "Container Condition", "Container Condition");
        table_0544.add_value("CC", "CC", false);
        table_0544.add_value("CL", "CL", false);
        table_0544.add_value("CT", "CT", false);
        table_0544.add_value("SB", "SB", false);
        table_0544.add_value("XAMB", "XAMB", false);
        table_0544.add_value("XC37", "XC37", false);
        table_0544.add_value("XCAMB", "XCAMB", false);
        table_0544.add_value("XCATM", "XCATM", false);
        table_0544.add_value("XCFRZ", "XCFRZ", false);
        table_0544.add_value("XCREF", "XCREF", false);
        table_0544.add_value("XDFRZ", "XDFRZ", false);
        table_0544.add_value("XDRY", "XDRY", false);
        table_0544.add_value("XFRZ", "XFRZ", false);
        table_0544.add_value("XMTLF", "XMTLF", false);
        table_0544.add_value("XNTR", "XNTR", false);
        table_0544.add_value("XPRTL", "XPRTL", false);
        table_0544.add_value("XPSA", "XPSA", false);
        table_0544.add_value("XPSO", "XPSO", false);
        table_0544.add_value("XREF", "XREF", false);
        table_0544.add_value("XUFRZ", "XUFRZ", false);
        table_0544.add_value("XUPR", "XUPR", false);
        self.add_table(table_0544);

        // HL7 Table 0548 - Signatory's Relationship to Subject
        let mut table_0548 = Hl7Table::new(
            "0548",
            "Signatory's Relationship to Subject",
            "Signatory's Relationship to Subject",
        );
        table_0548.add_value("1", "1", false);
        table_0548.add_value("2", "2", false);
        table_0548.add_value("3", "3", false);
        table_0548.add_value("4", "4", false);
        table_0548.add_value("5", "5", false);
        table_0548.add_value("6", "6", false);
        table_0548.add_value("7", "7", false);
        self.add_table(table_0548);

        // HL7 Table 0550 - Body Parts
        let mut table_0550 = Hl7Table::new("0550", "Body Parts", "Body Parts");
        table_0550.add_value("ACET", "ACET", false);
        table_0550.add_value("ACHIL", "ACHIL", false);
        table_0550.add_value("ADB", "ADB", false);
        table_0550.add_value("ADE", "ADE", false);
        table_0550.add_value("ADR", "ADR", false);
        table_0550.add_value("AMN", "AMN", false);
        table_0550.add_value("AMS", "AMS", false);
        table_0550.add_value("ANAL", "ANAL", false);
        table_0550.add_value("ANKL", "ANKL", false);
        table_0550.add_value("ANTEC", "ANTEC", false);
        table_0550.add_value("ANTECF", "ANTECF", false);
        table_0550.add_value("ANTR", "ANTR", false);
        table_0550.add_value("ANUS", "ANUS", false);
        table_0550.add_value("AORTA", "AORTA", false);
        table_0550.add_value("APDX", "APDX", false);
        table_0550.add_value("AR", "AR", false);
        table_0550.add_value("AREO", "AREO", false);
        table_0550.add_value("ARM", "ARM", false);
        table_0550.add_value("ARTE", "ARTE", false);
        table_0550.add_value("ASCIT", "ASCIT", false);
        table_0550.add_value("ASCT", "ASCT", false);
        table_0550.add_value("ATR", "ATR", false);
        table_0550.add_value("AURI", "AURI", false);
        table_0550.add_value("AV", "AV", false);
        table_0550.add_value("AXI", "AXI", false);
        table_0550.add_value("BACK", "BACK", false);
        table_0550.add_value("BARTD", "BARTD", false);
        table_0550.add_value("BARTG", "BARTG", false);
        table_0550.add_value("BCYS", "BCYS", false);
        table_0550.add_value("BDY", "BDY", false);
        table_0550.add_value("BID", "BID", false);
        table_0550.add_value("BIFL", "BIFL", false);
        table_0550.add_value("BLAD", "BLAD", false);
        table_0550.add_value("BLD", "BLD", false);
        table_0550.add_value("BLDA", "BLDA", false);
        table_0550.add_value("BLDC", "BLDC", false);
        table_0550.add_value("BLDV", "BLDV", false);
        table_0550.add_value("BLOOD", "BLOOD", false);
        table_0550.add_value("BMAR", "BMAR", false);
        table_0550.add_value("BON", "BON", false);
        table_0550.add_value("BOWEL", "BOWEL", false);
        table_0550.add_value("BOWLA", "BOWLA", false);
        table_0550.add_value("BOWSM", "BOWSM", false);
        table_0550.add_value("BPH", "BPH", false);
        table_0550.add_value("BRA", "BRA", false);
        table_0550.add_value("BRAIN", "BRAIN", false);
        table_0550.add_value("BRO", "BRO", false);
        table_0550.add_value("BROCH", "BROCH", false);
        table_0550.add_value("BRONC", "BRONC", false);
        table_0550.add_value("BROW", "BROW", false);
        table_0550.add_value("BRST", "BRST", false);
        table_0550.add_value("BRSTFL", "BRSTFL", false);
        table_0550.add_value("BRTGF", "BRTGF", false);
        table_0550.add_value("BRV", "BRV", false);
        table_0550.add_value("BUCCA", "BUCCA", false);
        table_0550.add_value("BURSA", "BURSA", false);
        table_0550.add_value("BURSF", "BURSF", false);
        table_0550.add_value("BUTT", "BUTT", false);
        table_0550.add_value("CALF", "CALF", false);
        table_0550.add_value("CANAL", "CANAL", false);
        table_0550.add_value("CANLI", "CANLI", false);
        table_0550.add_value("CANTH", "CANTH", false);
        table_0550.add_value("CARO", "CARO", false);
        table_0550.add_value("CARP", "CARP", false);
        table_0550.add_value("CAVIT", "CAVIT", false);
        table_0550.add_value("CBLD", "CBLD", false);
        table_0550.add_value("CDM", "CDM", false);
        table_0550.add_value("CDUCT", "CDUCT", false);
        table_0550.add_value("CECUM", "CECUM", false);
        table_0550.add_value("CERVUT", "CERVUT", false);
        table_0550.add_value("CHE", "CHE", false);
        table_0550.add_value("CHEEK", "CHEEK", false);
        table_0550.add_value("CHES", "CHES", false);
        table_0550.add_value("CHEST", "CHEST", false);
        table_0550.add_value("CHIN", "CHIN", false);
        table_0550.add_value("CIRCU", "CIRCU", false);
        table_0550.add_value("CLAVI", "CLAVI", false);
        table_0550.add_value("CLIT", "CLIT", false);
        table_0550.add_value("CLITO", "CLITO", false);
        table_0550.add_value("CNL", "CNL", false);
        table_0550.add_value("COCCG", "COCCG", false);
        table_0550.add_value("COCCY", "COCCY", false);
        table_0550.add_value("COLON", "COLON", false);
        table_0550.add_value("COLOS", "COLOS", false);
        table_0550.add_value("CONJ", "CONJ", false);
        table_0550.add_value("COR", "COR", false);
        table_0550.add_value("CORAL", "CORAL", false);
        table_0550.add_value("CORD", "CORD", false);
        table_0550.add_value("CORN", "CORN", false);
        table_0550.add_value("COS", "COS", false);
        table_0550.add_value("CRANE", "CRANE", false);
        table_0550.add_value("CRANF", "CRANF", false);
        table_0550.add_value("CRANO", "CRANO", false);
        table_0550.add_value("CRANP", "CRANP", false);
        table_0550.add_value("CRANS", "CRANS", false);
        table_0550.add_value("CRANT", "CRANT", false);
        table_0550.add_value("CSF", "CSF", false);
        table_0550.add_value("CUBIT", "CUBIT", false);
        table_0550.add_value("CUFF", "CUFF", false);
        table_0550.add_value("CULD", "CULD", false);
        table_0550.add_value("CULDO", "CULDO", false);
        table_0550.add_value("CVX", "CVX", false);
        table_0550.add_value("DELT", "DELT", false);
        table_0550.add_value("DEN", "DEN", false);
        table_0550.add_value("DENTA", "DENTA", false);
        table_0550.add_value("DIAF", "DIAF", false);
        table_0550.add_value("DIGIT", "DIGIT", false);
        table_0550.add_value("DISC", "DISC", false);
        table_0550.add_value("DORS", "DORS", false);
        table_0550.add_value("DPH", "DPH", false);
        table_0550.add_value("DUFL", "DUFL", false);
        table_0550.add_value("DUODE", "DUODE", false);
        table_0550.add_value("DUR", "DUR", false);
        table_0550.add_value("EAR", "EAR", false);
        table_0550.add_value("EARBI", "EARBI", false);
        table_0550.add_value("EARBM", "EARBM", false);
        table_0550.add_value("EARBS", "EARBS", false);
        table_0550.add_value("EARLO", "EARLO", false);
        table_0550.add_value("EC", "EC", false);
        table_0550.add_value("ELBOW", "ELBOW", false);
        table_0550.add_value("ELBOWJ", "ELBOWJ", false);
        table_0550.add_value("ENDC", "ENDC", false);
        table_0550.add_value("ENDM", "ENDM", false);
        table_0550.add_value("EOLPH", "EOLPH", false);
        table_0550.add_value("EOS", "EOS", false);
        table_0550.add_value("EPD", "EPD", false);
        table_0550.add_value("EPICA", "EPICA", false);
        table_0550.add_value("EPICM", "EPICM", false);
        table_0550.add_value("EPIDU", "EPIDU", false);
        table_0550.add_value("EPIGL", "EPIGL", false);
        table_0550.add_value("ESO", "ESO", false);
        table_0550.add_value("ESOPG", "ESOPG", false);
        table_0550.add_value("ET", "ET", false);
        table_0550.add_value("ETHMO", "ETHMO", false);
        table_0550.add_value("EUR", "EUR", false);
        table_0550.add_value("EYE", "EYE", false);
        table_0550.add_value("EYELI", "EYELI", false);
        table_0550.add_value("FACE", "FACE", false);
        table_0550.add_value("FALLT", "FALLT", false);
        table_0550.add_value("FBINC", "FBINC", false);
        table_0550.add_value("FBLAC", "FBLAC", false);
        table_0550.add_value("FBMAX", "FBMAX", false);
        table_0550.add_value("FBNAS", "FBNAS", false);
        table_0550.add_value("FBPAL", "FBPAL", false);
        table_0550.add_value("FBVOM", "FBVOM", false);
        table_0550.add_value("FBZYG", "FBZYG", false);
        table_0550.add_value("FEMOR", "FEMOR", false);
        table_0550.add_value("FEMUR", "FEMUR", false);
        table_0550.add_value("FET", "FET", false);
        table_0550.add_value("FIBU", "FIBU", false);
        table_0550.add_value("FING", "FING", false);
        table_0550.add_value("FINGN", "FINGN", false);
        table_0550.add_value("FMH", "FMH", false);
        table_0550.add_value("FOL", "FOL", false);
        table_0550.add_value("FOOT", "FOOT", false);
        table_0550.add_value("FOREA", "FOREA", false);
        table_0550.add_value("FOREH", "FOREH", false);
        table_0550.add_value("FORES", "FORES", false);
        table_0550.add_value("FOURC", "FOURC", false);
        table_0550.add_value("GB", "GB", false);
        table_0550.add_value("GEN", "GEN", false);
        table_0550.add_value("GENC", "GENC", false);
        table_0550.add_value("GENL", "GENL", false);
        table_0550.add_value("GL", "GL", false);
        table_0550.add_value("GLAND", "GLAND", false);
        table_0550.add_value("GLANS", "GLANS", false);
        table_0550.add_value("GLUT", "GLUT", false);
        table_0550.add_value("GLUTE", "GLUTE", false);
        table_0550.add_value("GLUTM", "GLUTM", false);
        table_0550.add_value("GROIN", "GROIN", false);
        table_0550.add_value("GUM", "GUM", false);
        table_0550.add_value("GVU", "GVU", false);
        table_0550.add_value("HAL", "HAL", false);
        table_0550.add_value("HAND", "HAND", false);
        table_0550.add_value("HAR", "HAR", false);
        table_0550.add_value("HART", "HART", false);
        table_0550.add_value("HEAD", "HEAD", false);
        table_0550.add_value("HEEL", "HEEL", false);
        table_0550.add_value("HEM", "HEM", false);
        table_0550.add_value("HIP", "HIP", false);
        table_0550.add_value("HIPJ", "HIPJ", false);
        table_0550.add_value("HUMER", "HUMER", false);
        table_0550.add_value("HV", "HV", false);
        table_0550.add_value("HVB", "HVB", false);
        table_0550.add_value("HVT", "HVT", false);
        table_0550.add_value("HYMEN", "HYMEN", false);
        table_0550.add_value("ICX", "ICX", false);
        table_0550.add_value("ILC", "ILC", false);
        table_0550.add_value("ILCON", "ILCON", false);
        table_0550.add_value("ILCR", "ILCR", false);
        table_0550.add_value("ILE", "ILE", false);
        table_0550.add_value("ILEOS", "ILEOS", false);
        table_0550.add_value("ILEUM", "ILEUM", false);
        table_0550.add_value("ILIAC", "ILIAC", false);
        table_0550.add_value("INASA", "INASA", false);
        table_0550.add_value("INGUI", "INGUI", false);
        table_0550.add_value("INSTL", "INSTL", false);
        table_0550.add_value("INSTS", "INSTS", false);
        table_0550.add_value("INT", "INT", false);
        table_0550.add_value("INTRO", "INTRO", false);
        table_0550.add_value("INTRU", "INTRU", false);
        table_0550.add_value("ISCHI", "ISCHI", false);
        table_0550.add_value("ISH", "ISH", false);
        table_0550.add_value("JAW", "JAW", false);
        table_0550.add_value("JUGI", "JUGI", false);
        table_0550.add_value("KIDN", "KIDN", false);
        table_0550.add_value("KNEE", "KNEE", false);
        table_0550.add_value("KNEEF", "KNEEF", false);
        table_0550.add_value("KNEEJ", "KNEEJ", false);
        table_0550.add_value("LABIA", "LABIA", false);
        table_0550.add_value("LABMA", "LABMA", false);
        table_0550.add_value("LABMI", "LABMI", false);
        table_0550.add_value("LACRI", "LACRI", false);
        table_0550.add_value("LAM", "LAM", false);
        table_0550.add_value("LARYN", "LARYN", false);
        table_0550.add_value("LEG", "LEG", false);
        table_0550.add_value("LENS", "LENS", false);
        table_0550.add_value("LING", "LING", false);
        table_0550.add_value("LINGU", "LINGU", false);
        table_0550.add_value("LIP", "LIP", false);
        table_0550.add_value("LIVER", "LIVER", false);
        table_0550.add_value("LMN", "LMN", false);
        table_0550.add_value("LN", "LN", false);
        table_0550.add_value("LNG", "LNG", false);
        table_0550.add_value("LOBE", "LOBE", false);
        table_0550.add_value("LOCH", "LOCH", false);
        table_0550.add_value("LUMBA", "LUMBA", false);
        table_0550.add_value("LUNG", "LUNG", false);
        table_0550.add_value("LYM", "LYM", false);
        table_0550.add_value("MAC", "MAC", false);
        table_0550.add_value("MALLE", "MALLE", false);
        table_0550.add_value("MANDI", "MANDI", false);
        table_0550.add_value("MAR", "MAR", false);
        table_0550.add_value("MAST", "MAST", false);
        table_0550.add_value("MAXIL", "MAXIL", false);
        table_0550.add_value("MAXS", "MAXS", false);
        table_0550.add_value("MEATU", "MEATU", false);
        table_0550.add_value("MEC", "MEC", false);
        table_0550.add_value("MEDST", "MEDST", false);
        table_0550.add_value("MEDU", "MEDU", false);
        table_0550.add_value("METAC", "METAC", false);
        table_0550.add_value("METAT", "METAT", false);
        table_0550.add_value("MILK", "MILK", false);
        table_0550.add_value("MITRL", "MITRL", false);
        table_0550.add_value("MOLAR", "MOLAR", false);
        table_0550.add_value("MONSU", "MONSU", false);
        table_0550.add_value("MONSV", "MONSV", false);
        table_0550.add_value("MOU", "MOU", false);
        table_0550.add_value("MOUTH", "MOUTH", false);
        table_0550.add_value("MP", "MP", false);
        table_0550.add_value("MPB", "MPB", false);
        table_0550.add_value("MRSA2", "MRSA2", false);
        table_0550.add_value("MYO", "MYO", false);
        table_0550.add_value("NAIL", "NAIL", false);
        table_0550.add_value("NAILB", "NAILB", false);
        table_0550.add_value("NAILF", "NAILF", false);
        table_0550.add_value("NAILT", "NAILT", false);
        table_0550.add_value("NARES", "NARES", false);
        table_0550.add_value("NASL", "NASL", false);
        table_0550.add_value("NAVEL", "NAVEL", false);
        table_0550.add_value("NECK", "NECK", false);
        table_0550.add_value("NERVE", "NERVE", false);
        table_0550.add_value("NIPPL", "NIPPL", false);
        table_0550.add_value("NLACR", "NLACR", false);
        table_0550.add_value("NOS", "NOS", false);
        table_0550.add_value("NOSE", "NOSE", false);
        table_0550.add_value("NOSTR", "NOSTR", false);
        table_0550.add_value("NP", "NP", false);
        table_0550.add_value("NSS", "NSS", false);
        table_0550.add_value("NTRAC", "NTRAC", false);
        table_0550.add_value("OCCIP", "OCCIP", false);
        table_0550.add_value("OLECR", "OLECR", false);
        table_0550.add_value("OMEN", "OMEN", false);
        table_0550.add_value("ORBIT", "ORBIT", false);
        table_0550.add_value("ORO", "ORO", false);
        table_0550.add_value("OSCOX", "OSCOX", false);
        table_0550.add_value("OVARY", "OVARY", false);
        table_0550.add_value("PAFL", "PAFL", false);
        table_0550.add_value("PALAT", "PALAT", false);
        table_0550.add_value("PALM", "PALM", false);
        table_0550.add_value("PANAL", "PANAL", false);
        table_0550.add_value("PANCR", "PANCR", false);
        table_0550.add_value("PARAT", "PARAT", false);
        table_0550.add_value("PARIE", "PARIE", false);
        table_0550.add_value("PARON", "PARON", false);
        table_0550.add_value("PAROT", "PAROT", false);
        table_0550.add_value("PAS", "PAS", false);
        table_0550.add_value("PATEL", "PATEL", false);
        table_0550.add_value("PCARD", "PCARD", false);
        table_0550.add_value("PCLIT", "PCLIT", false);
        table_0550.add_value("PELV", "PELV", false);
        table_0550.add_value("PENIS", "PENIS", false);
        table_0550.add_value("PENSH", "PENSH", false);
        table_0550.add_value("PER", "PER", false);
        table_0550.add_value("PERI", "PERI", false);
        table_0550.add_value("PERIH", "PERIH", false);
        table_0550.add_value("PERIN", "PERIN", false);
        table_0550.add_value("PERIS", "PERIS", false);
        table_0550.add_value("PERIT", "PERIT", false);
        table_0550.add_value("PERIU", "PERIU", false);
        table_0550.add_value("PERIV", "PERIV", false);
        table_0550.add_value("PERRA", "PERRA", false);
        table_0550.add_value("PERT", "PERT", false);
        table_0550.add_value("PHALA", "PHALA", false);
        table_0550.add_value("PILO", "PILO", false);
        table_0550.add_value("PINNA", "PINNA", false);
        table_0550.add_value("PLACF", "PLACF", false);
        table_0550.add_value("PLACM", "PLACM", false);
        table_0550.add_value("PLANT", "PLANT", false);
        table_0550.add_value("PLATH", "PLATH", false);
        table_0550.add_value("PLATS", "PLATS", false);
        table_0550.add_value("PLC", "PLC", false);
        table_0550.add_value("PLEU", "PLEU", false);
        table_0550.add_value("PLEUR", "PLEUR", false);
        table_0550.add_value("PLR", "PLR", false);
        table_0550.add_value("PNEAL", "PNEAL", false);
        table_0550.add_value("PNEPH", "PNEPH", false);
        table_0550.add_value("PNM", "PNM", false);
        table_0550.add_value("POPLI", "POPLI", false);
        table_0550.add_value("PORBI", "PORBI", false);
        table_0550.add_value("PREAU", "PREAU", false);
        table_0550.add_value("PRERE", "PRERE", false);
        table_0550.add_value("PROS", "PROS", false);
        table_0550.add_value("PRST", "PRST", false);
        table_0550.add_value("PTONS", "PTONS", false);
        table_0550.add_value("PUBIC", "PUBIC", false);
        table_0550.add_value("PUL", "PUL", false);
        table_0550.add_value("RADI", "RADI", false);
        table_0550.add_value("RADIUS", "RADIUS", false);
        table_0550.add_value("RBC", "RBC", false);
        table_0550.add_value("RECTL", "RECTL", false);
        table_0550.add_value("RECTU", "RECTU", false);
        table_0550.add_value("RENL", "RENL", false);
        table_0550.add_value("RIB", "RIB", false);
        table_0550.add_value("RNP", "RNP", false);
        table_0550.add_value("RPERI", "RPERI", false);
        table_0550.add_value("SAC", "SAC", false);
        table_0550.add_value("SACIL", "SACIL", false);
        table_0550.add_value("SACRA", "SACRA", false);
        table_0550.add_value("SACRO", "SACRO", false);
        table_0550.add_value("SACRU", "SACRU", false);
        table_0550.add_value("SALGL", "SALGL", false);
        table_0550.add_value("SCALP", "SCALP", false);
        table_0550.add_value("SCAPU", "SCAPU", false);
        table_0550.add_value("SCLAV", "SCLAV", false);
        table_0550.add_value("SCLER", "SCLER", false);
        table_0550.add_value("SCLV", "SCLV", false);
        table_0550.add_value("SCROT", "SCROT", false);
        table_0550.add_value("SDP", "SDP", false);
        table_0550.add_value("SEM", "SEM", false);
        table_0550.add_value("SEMN", "SEMN", false);
        table_0550.add_value("SEPTU", "SEPTU", false);
        table_0550.add_value("SEROM", "SEROM", false);
        table_0550.add_value("SGF", "SGF", false);
        table_0550.add_value("SHIN", "SHIN", false);
        table_0550.add_value("SHOL", "SHOL", false);
        table_0550.add_value("SHOLJ", "SHOLJ", false);
        table_0550.add_value("SIGMO", "SIGMO", false);
        table_0550.add_value("SINUS", "SINUS", false);
        table_0550.add_value("SKENE", "SKENE", false);
        table_0550.add_value("SKM", "SKM", false);
        table_0550.add_value("SKULL", "SKULL", false);
        table_0550.add_value("SOLE", "SOLE", false);
        table_0550.add_value("SPCOR", "SPCOR", false);
        table_0550.add_value("SPHEN", "SPHEN", false);
        table_0550.add_value("SPLN", "SPLN", false);
        table_0550.add_value("SPRM", "SPRM", false);
        table_0550.add_value("SPX", "SPX", false);
        table_0550.add_value("STER", "STER", false);
        table_0550.add_value("STOM", "STOM", false);
        table_0550.add_value("STOMA", "STOMA", false);
        table_0550.add_value("STOOLL", "STOOLL", false);
        table_0550.add_value("STUMP", "STUMP", false);
        table_0550.add_value("SUB", "SUB", false);
        table_0550.add_value("SUBD", "SUBD", false);
        table_0550.add_value("SUBM", "SUBM", false);
        table_0550.add_value("SUBME", "SUBME", false);
        table_0550.add_value("SUBPH", "SUBPH", false);
        table_0550.add_value("SUBX", "SUBX", false);
        table_0550.add_value("SUPB", "SUPB", false);
        table_0550.add_value("SUPRA", "SUPRA", false);
        table_0550.add_value("SWT", "SWT", false);
        table_0550.add_value("SWTG", "SWTG", false);
        table_0550.add_value("SYN", "SYN", false);
        table_0550.add_value("SYNOL", "SYNOL", false);
        table_0550.add_value("SYNOV", "SYNOV", false);
        table_0550.add_value("TARS", "TARS", false);
        table_0550.add_value("TBRON", "TBRON", false);
        table_0550.add_value("TCN", "TCN", false);
        table_0550.add_value("TDUCT", "TDUCT", false);
        table_0550.add_value("TEAR", "TEAR", false);
        table_0550.add_value("TEMPL", "TEMPL", false);
        table_0550.add_value("TEMPO", "TEMPO", false);
        table_0550.add_value("TESTI", "TESTI", false);
        table_0550.add_value("THIGH", "THIGH", false);
        table_0550.add_value("THM", "THM", false);
        table_0550.add_value("THORA", "THORA", false);
        table_0550.add_value("THRB", "THRB", false);
        table_0550.add_value("THUMB", "THUMB", false);
        table_0550.add_value("THYRD", "THYRD", false);
        table_0550.add_value("TIBIA", "TIBIA", false);
        table_0550.add_value("TML", "TML", false);
        table_0550.add_value("TNL", "TNL", false);
        table_0550.add_value("TOE", "TOE", false);
        table_0550.add_value("TOEN", "TOEN", false);
        table_0550.add_value("TONG", "TONG", false);
        table_0550.add_value("TONS", "TONS", false);
        table_0550.add_value("TOOTH", "TOOTH", false);
        table_0550.add_value("TRCHE", "TRCHE", false);
        table_0550.add_value("TSK", "TSK", false);
        table_0550.add_value("ULNA", "ULNA", false);
        table_0550.add_value("UMB", "UMB", false);
        table_0550.add_value("UMBL", "UMBL", false);
        table_0550.add_value("URET", "URET", false);
        table_0550.add_value("URTH", "URTH", false);
        table_0550.add_value("USTOM", "USTOM", false);
        table_0550.add_value("UTER", "UTER", false);
        table_0550.add_value("UTERI", "UTERI", false);
        table_0550.add_value("VAGIN", "VAGIN", false);
        table_0550.add_value("VAL", "VAL", false);
        table_0550.add_value("VAS", "VAS", false);
        table_0550.add_value("VASTL", "VASTL", false);
        table_0550.add_value("VAULT", "VAULT", false);
        table_0550.add_value("VCSF", "VCSF", false);
        table_0550.add_value("VCUFF", "VCUFF", false);
        table_0550.add_value("VEIN", "VEIN", false);
        table_0550.add_value("VENTG", "VENTG", false);
        table_0550.add_value("VERMI", "VERMI", false);
        table_0550.add_value("VERTC", "VERTC", false);
        table_0550.add_value("VERTL", "VERTL", false);
        table_0550.add_value("VERTT", "VERTT", false);
        table_0550.add_value("VESCL", "VESCL", false);
        table_0550.add_value("VESFLD", "VESFLD", false);
        table_0550.add_value("VESI", "VESI", false);
        table_0550.add_value("VESTI", "VESTI", false);
        table_0550.add_value("VGV", "VGV", false);
        table_0550.add_value("VITR", "VITR", false);
        table_0550.add_value("VOC", "VOC", false);
        table_0550.add_value("VULVA", "VULVA", false);
        table_0550.add_value("WBC", "WBC", false);
        table_0550.add_value("WRIST", "WRIST", false);
        self.add_table(table_0550);

        // HL7 Table 0615 - User Authentication Credential Type Code
        let mut table_0615 = Hl7Table::new(
            "0615",
            "User Authentication Credential Type Code",
            "User Authentication Credential Type Code",
        );
        table_0615.add_value("KERB", "KERB", false);
        table_0615.add_value("SAML", "SAML", false);
        self.add_table(table_0615);

        // HL7 Table 0717 - Access Restriction Value
        let mut table_0717 = Hl7Table::new(
            "0717",
            "Access Restriction Value",
            "Access Restriction Value",
        );
        table_0717.add_value("ALL", "ALL", false);
        table_0717.add_value("DEM", "DEM", false);
        table_0717.add_value("DRG", "DRG", false);
        table_0717.add_value("HIV", "HIV", false);
        table_0717.add_value("LOC", "LOC", false);
        table_0717.add_value("NO", "NO", false);
        table_0717.add_value("OI", "OI", false);
        table_0717.add_value("OO", "OO", false);
        table_0717.add_value("PID-17", "PID-17", false);
        table_0717.add_value("PID-7", "PID-7", false);
        table_0717.add_value("PSY", "PSY", false);
        table_0717.add_value("SMD", "SMD", false);
        table_0717.add_value("STD", "STD", false);
        self.add_table(table_0717);

        // HL7 Table 0719 - Access Restriction Reason Code
        let mut table_0719 = Hl7Table::new(
            "0719",
            "Access Restriction Reason Code",
            "Access Restriction Reason Code",
        );
        table_0719.add_value("DIA", "DIA", false);
        table_0719.add_value("EMP", "EMP", false);
        table_0719.add_value("ORG", "ORG", false);
        table_0719.add_value("PAT", "PAT", false);
        table_0719.add_value("PHY", "PHY", false);
        table_0719.add_value("REG", "REG", false);
        table_0719.add_value("VIP", "VIP", false);
        self.add_table(table_0719);

        // HL7 Table 0725 - Mood Codes
        let mut table_0725 = Hl7Table::new("0725", "Mood Codes", "Mood Codes");
        table_0725.add_value("APT", "APT", false);
        table_0725.add_value("ARQ", "ARQ", false);
        table_0725.add_value("EVN", "EVN", false);
        table_0725.add_value("EVN.CRT", "EVN.CRT", false);
        table_0725.add_value("EXP", "EXP", false);
        table_0725.add_value("INT", "INT", false);
        table_0725.add_value("PRMS", "PRMS", false);
        table_0725.add_value("PRP", "PRP", false);
        table_0725.add_value("RQO", "RQO", false);
        table_0725.add_value("A", "A", false);
        table_0725.add_value("I", "I", false);
        table_0725.add_value("P", "P", false);
        self.add_table(table_0725);

        // HL7 Table 0728 - CCL Value
        let mut table_0728 = Hl7Table::new("0728", "CCL Value", "CCL Value");
        table_0728.add_value("0", "0", false);
        table_0728.add_value("1", "1", false);
        table_0728.add_value("2", "2", false);
        table_0728.add_value("3", "3", false);
        table_0728.add_value("4", "4", false);
        self.add_table(table_0728);

        // HL7 Table 0731 - DRG Diagnosis Determination Status
        let mut table_0731 = Hl7Table::new(
            "0731",
            "DRG Diagnosis Determination Status",
            "DRG Diagnosis Determination Status",
        );
        table_0731.add_value("0", "0", false);
        table_0731.add_value("1", "1", false);
        table_0731.add_value("2", "2", false);
        table_0731.add_value("3", "3", false);
        table_0731.add_value("4", "4", false);
        self.add_table(table_0731);

        // HL7 Table 0734 - Grouper Status
        let mut table_0734 = Hl7Table::new("0734", "Grouper Status", "Grouper Status");
        table_0734.add_value("0", "0", false);
        table_0734.add_value("1", "1", false);
        table_0734.add_value("2", "2", false);
        table_0734.add_value("3", "3", false);
        table_0734.add_value("4", "4", false);
        table_0734.add_value("5", "5", false);
        table_0734.add_value("6", "6", false);
        table_0734.add_value("7", "7", false);
        table_0734.add_value("8", "8", false);
        table_0734.add_value("9", "9", false);
        self.add_table(table_0734);

        // HL7 Table 0739 - DRG Status Patient
        let mut table_0739 = Hl7Table::new("0739", "DRG Status Patient", "DRG Status Patient");
        table_0739.add_value("1", "1", false);
        table_0739.add_value("2", "2", false);
        table_0739.add_value("3", "3", false);
        self.add_table(table_0739);

        // HL7 Table 0742 - DRG Status Financial Calculation
        let mut table_0742 = Hl7Table::new(
            "0742",
            "DRG Status Financial Calculation",
            "DRG Status Financial Calculation",
        );
        table_0742.add_value("0", "0", false);
        table_0742.add_value("1", "1", false);
        table_0742.add_value("10", "10", false);
        table_0742.add_value("11", "11", false);
        table_0742.add_value("3", "3", false);
        table_0742.add_value("4", "4", false);
        table_0742.add_value("5", "5", false);
        self.add_table(table_0742);

        // HL7 Table 0749 - DRG Grouping Status
        let mut table_0749 = Hl7Table::new("0749", "DRG Grouping Status", "DRG Grouping Status");
        table_0749.add_value("0", "0", false);
        table_0749.add_value("1", "1", false);
        table_0749.add_value("2", "2", false);
        table_0749.add_value("3", "3", false);
        self.add_table(table_0749);

        // HL7 Table 0755 - Status Weight At Birth
        let mut table_0755 =
            Hl7Table::new("0755", "Status Weight At Birth", "Status Weight At Birth");
        table_0755.add_value("0", "0", false);
        table_0755.add_value("1", "1", false);
        table_0755.add_value("2", "2", false);
        self.add_table(table_0755);

        // HL7 Table 0757 - DRG Status Respiration Minutes
        let mut table_0757 = Hl7Table::new(
            "0757",
            "DRG Status Respiration Minutes",
            "DRG Status Respiration Minutes",
        );
        table_0757.add_value("0", "0", false);
        table_0757.add_value("1", "1", false);
        table_0757.add_value("2", "2", false);
        self.add_table(table_0757);

        // HL7 Table 0759 - Status Admission
        let mut table_0759 = Hl7Table::new("0759", "Status Admission", "Status Admission");
        table_0759.add_value("0", "0", false);
        table_0759.add_value("1", "1", false);
        table_0759.add_value("2", "2", false);
        table_0759.add_value("3", "3", false);
        self.add_table(table_0759);

        // HL7 Table 0761 - DRG Procedure Determination Status
        let mut table_0761 = Hl7Table::new(
            "0761",
            "DRG Procedure Determination Status",
            "DRG Procedure Determination Status",
        );
        table_0761.add_value("0", "0", false);
        table_0761.add_value("1", "1", false);
        table_0761.add_value("2", "2", false);
        table_0761.add_value("3", "3", false);
        table_0761.add_value("4", "4", false);
        self.add_table(table_0761);

        // HL7 Table 0763 - DRG Procedure Relevance
        let mut table_0763 =
            Hl7Table::new("0763", "DRG Procedure Relevance", "DRG Procedure Relevance");
        table_0763.add_value("0", "0", false);
        table_0763.add_value("1", "1", false);
        table_0763.add_value("2", "2", false);
        self.add_table(table_0763);

        // HL7 Table 0895 - Present On Admission (POA) Indicator
        let mut table_0895 = Hl7Table::new(
            "0895",
            "Present On Admission (POA) Indicator",
            "Present On Admission (POA) Indicator",
        );
        table_0895.add_value("E", "E", false);
        table_0895.add_value("N", "N", false);
        table_0895.add_value("U", "U", false);
        table_0895.add_value("W", "W", false);
        table_0895.add_value("Y", "Y", false);
        self.add_table(table_0895);

        // HL7 Table 0909 - Patient Results Release Categorization Scheme
        let mut table_0909 = Hl7Table::new(
            "0909",
            "Patient Results Release Categorization Scheme",
            "Patient Results Release Categorization Scheme",
        );
        table_0909.add_value("SID", "SID", false);
        table_0909.add_value("SIDC", "SIDC", false);
        table_0909.add_value("SIMM", "SIMM", false);
        table_0909.add_value("STBD", "STBD", false);
        table_0909.add_value("SWNL", "SWNL", false);
        table_0909.add_value("SWTH", "SWTH", false);
        self.add_table(table_0909);

        // HL7 Table 0912 - Participation
        let mut table_0912 = Hl7Table::new("0912", "Participation", "Participation");
        table_0912.add_value("AAP", "AAP", false);
        table_0912.add_value("AC", "AC", false);
        table_0912.add_value("AD", "AD", false);
        table_0912.add_value("AI", "AI", false);
        table_0912.add_value("AP", "AP", false);
        table_0912.add_value("ARI", "ARI", false);
        table_0912.add_value("AT", "AT", false);
        table_0912.add_value("AUT", "AUT", false);
        table_0912.add_value("CP", "CP", false);
        table_0912.add_value("DP", "DP", false);
        table_0912.add_value("EP", "EP", false);
        table_0912.add_value("EQUIP", "EQUIP", false);
        table_0912.add_value("FHCP", "FHCP", false);
        table_0912.add_value("MDIR", "MDIR", false);
        table_0912.add_value("OP", "OP", false);
        table_0912.add_value("OS", "OS", false);
        table_0912.add_value("PB", "PB", false);
        table_0912.add_value("PH", "PH", false);
        table_0912.add_value("PI", "PI", false);
        table_0912.add_value("PO", "PO", false);
        table_0912.add_value("POMD", "POMD", false);
        table_0912.add_value("PP", "PP", false);
        table_0912.add_value("PRI", "PRI", false);
        table_0912.add_value("RCT", "RCT", false);
        table_0912.add_value("RO", "RO", false);
        table_0912.add_value("RP", "RP", false);
        table_0912.add_value("RT", "RT", false);
        table_0912.add_value("SB", "SB", false);
        table_0912.add_value("SC", "SC", false);
        table_0912.add_value("TN", "TN", false);
        table_0912.add_value("TR", "TR", false);
        table_0912.add_value("VP", "VP", false);
        table_0912.add_value("VPS", "VPS", false);
        table_0912.add_value("VTS", "VTS", false);
        table_0912.add_value("WAY", "WAY", false);
        table_0912.add_value("WAYR", "WAYR", false);
        self.add_table(table_0912);

        // HL7 Table 0914 - Root Cause
        let mut table_0914 = Hl7Table::new("0914", "Root Cause", "Root Cause");
        table_0914.add_value("AP", "AP", false);
        table_0914.add_value("IM", "IM", false);
        table_0914.add_value("L", "L", false);
        table_0914.add_value("NA", "NA", false);
        table_0914.add_value("PD", "PD", false);
        self.add_table(table_0914);

        // HL7 Table 0916 - Relevant Clincial Information
        let mut table_0916 = Hl7Table::new(
            "0916",
            "Relevant Clincial Information",
            "Relevant Clincial Information",
        );
        table_0916.add_value("F", "F", false);
        table_0916.add_value("NF", "NF", false);
        table_0916.add_value("NG", "NG", false);
        self.add_table(table_0916);

        // HL7 Table 0917 - Bolus Type
        let mut table_0917 = Hl7Table::new("0917", "Bolus Type", "Bolus Type");
        table_0917.add_value("C", "C", false);
        table_0917.add_value("L", "L", false);
        self.add_table(table_0917);

        // HL7 Table 0918 - PCA Type
        let mut table_0918 = Hl7Table::new("0918", "PCA Type", "PCA Type");
        table_0918.add_value("C", "C", false);
        table_0918.add_value("P", "P", false);
        table_0918.add_value("PC", "PC", false);
        self.add_table(table_0918);

        // HL7 Table 0921 - Certification Type Code
        let mut table_0921 =
            Hl7Table::new("0921", "Certification Type Code", "Certification Type Code");
        table_0921.add_value("ADM", "ADM", false);
        table_0921.add_value("PROC", "PROC", false);
        table_0921.add_value("SERV", "SERV", false);
        self.add_table(table_0921);

        // HL7 Table 0922 - Certification Category Code
        let mut table_0922 = Hl7Table::new(
            "0922",
            "Certification Category Code",
            "Certification Category Code",
        );
        table_0922.add_value("IR", "IR", false);
        table_0922.add_value("RA", "RA", false);
        table_0922.add_value("RE", "RE", false);
        self.add_table(table_0922);

        // HL7 Table 0924 - Cumulative Dosage Limit UoM
        let mut table_0924 = Hl7Table::new(
            "0924",
            "Cumulative Dosage Limit UoM",
            "Cumulative Dosage Limit UoM",
        );
        table_0924.add_value("A", "A", false);
        table_0924.add_value("D", "D", false);
        table_0924.add_value("M", "M", false);
        table_0924.add_value("O", "O", false);
        table_0924.add_value("PL", "PL", false);
        table_0924.add_value("WK", "WK", false);
        self.add_table(table_0924);

        // HL7 Table 4000 - Name/address representation
        let mut table_4000 = Hl7Table::new(
            "4000",
            "Name/address representation",
            "Name/address representation",
        );
        table_4000.add_value("A", "A", false);
        table_4000.add_value("I", "I", false);
        table_4000.add_value("P", "P", false);
        self.add_table(table_4000);

        // ISO 4217 - Currency Codes (hand-maintained; active alpha-3 codes).
        // Referenced by MO/CP currency components; descriptions intentionally
        // mirror codes (cf. generated union-only tables).
        let mut table_ISO4217 = Hl7Table::new(
            "ISO4217",
            "Currency Codes",
            "ISO 4217 active currency codes",
        );
        table_ISO4217.add_value("AED", "AED", false);
        table_ISO4217.add_value("AFN", "AFN", false);
        table_ISO4217.add_value("ALL", "ALL", false);
        table_ISO4217.add_value("AMD", "AMD", false);
        table_ISO4217.add_value("ANG", "ANG", false);
        table_ISO4217.add_value("AOA", "AOA", false);
        table_ISO4217.add_value("ARS", "ARS", false);
        table_ISO4217.add_value("AUD", "AUD", false);
        table_ISO4217.add_value("AWG", "AWG", false);
        table_ISO4217.add_value("AZN", "AZN", false);
        table_ISO4217.add_value("BAM", "BAM", false);
        table_ISO4217.add_value("BBD", "BBD", false);
        table_ISO4217.add_value("BDT", "BDT", false);
        table_ISO4217.add_value("BGN", "BGN", false);
        table_ISO4217.add_value("BHD", "BHD", false);
        table_ISO4217.add_value("BIF", "BIF", false);
        table_ISO4217.add_value("BMD", "BMD", false);
        table_ISO4217.add_value("BND", "BND", false);
        table_ISO4217.add_value("BOB", "BOB", false);
        table_ISO4217.add_value("BOV", "BOV", false);
        table_ISO4217.add_value("BRL", "BRL", false);
        table_ISO4217.add_value("BSD", "BSD", false);
        table_ISO4217.add_value("BTN", "BTN", false);
        table_ISO4217.add_value("BWP", "BWP", false);
        table_ISO4217.add_value("BYN", "BYN", false);
        table_ISO4217.add_value("BZD", "BZD", false);
        table_ISO4217.add_value("CAD", "CAD", false);
        table_ISO4217.add_value("CDF", "CDF", false);
        table_ISO4217.add_value("CHE", "CHE", false);
        table_ISO4217.add_value("CHF", "CHF", false);
        table_ISO4217.add_value("CHW", "CHW", false);
        table_ISO4217.add_value("CLF", "CLF", false);
        table_ISO4217.add_value("CLP", "CLP", false);
        table_ISO4217.add_value("CNY", "CNY", false);
        table_ISO4217.add_value("COP", "COP", false);
        table_ISO4217.add_value("COU", "COU", false);
        table_ISO4217.add_value("CRC", "CRC", false);
        table_ISO4217.add_value("CUP", "CUP", false);
        table_ISO4217.add_value("CVE", "CVE", false);
        table_ISO4217.add_value("CZK", "CZK", false);
        table_ISO4217.add_value("DJF", "DJF", false);
        table_ISO4217.add_value("DKK", "DKK", false);
        table_ISO4217.add_value("DOP", "DOP", false);
        table_ISO4217.add_value("DZD", "DZD", false);
        table_ISO4217.add_value("EGP", "EGP", false);
        table_ISO4217.add_value("ERN", "ERN", false);
        table_ISO4217.add_value("ETB", "ETB", false);
        table_ISO4217.add_value("EUR", "EUR", false);
        table_ISO4217.add_value("FJD", "FJD", false);
        table_ISO4217.add_value("FKP", "FKP", false);
        table_ISO4217.add_value("GBP", "GBP", false);
        table_ISO4217.add_value("GEL", "GEL", false);
        table_ISO4217.add_value("GGP", "GGP", false);
        table_ISO4217.add_value("GHS", "GHS", false);
        table_ISO4217.add_value("GIP", "GIP", false);
        table_ISO4217.add_value("GMD", "GMD", false);
        table_ISO4217.add_value("GNF", "GNF", false);
        table_ISO4217.add_value("GTQ", "GTQ", false);
        table_ISO4217.add_value("GYD", "GYD", false);
        table_ISO4217.add_value("HKD", "HKD", false);
        table_ISO4217.add_value("HNL", "HNL", false);
        table_ISO4217.add_value("HTG", "HTG", false);
        table_ISO4217.add_value("HUF", "HUF", false);
        table_ISO4217.add_value("IDR", "IDR", false);
        table_ISO4217.add_value("ILS", "ILS", false);
        table_ISO4217.add_value("IMP", "IMP", false);
        table_ISO4217.add_value("INR", "INR", false);
        table_ISO4217.add_value("IQD", "IQD", false);
        table_ISO4217.add_value("IRR", "IRR", false);
        table_ISO4217.add_value("ISK", "ISK", false);
        table_ISO4217.add_value("JEP", "JEP", false);
        table_ISO4217.add_value("JMD", "JMD", false);
        table_ISO4217.add_value("JOD", "JOD", false);
        table_ISO4217.add_value("JPY", "JPY", false);
        table_ISO4217.add_value("KES", "KES", false);
        table_ISO4217.add_value("KGS", "KGS", false);
        table_ISO4217.add_value("KHR", "KHR", false);
        table_ISO4217.add_value("KMF", "KMF", false);
        table_ISO4217.add_value("KPW", "KPW", false);
        table_ISO4217.add_value("KRW", "KRW", false);
        table_ISO4217.add_value("KWD", "KWD", false);
        table_ISO4217.add_value("KYD", "KYD", false);
        table_ISO4217.add_value("KZT", "KZT", false);
        table_ISO4217.add_value("LAK", "LAK", false);
        table_ISO4217.add_value("LBP", "LBP", false);
        table_ISO4217.add_value("LKR", "LKR", false);
        table_ISO4217.add_value("LRD", "LRD", false);
        table_ISO4217.add_value("LSL", "LSL", false);
        table_ISO4217.add_value("LYD", "LYD", false);
        table_ISO4217.add_value("MAD", "MAD", false);
        table_ISO4217.add_value("MDL", "MDL", false);
        table_ISO4217.add_value("MGA", "MGA", false);
        table_ISO4217.add_value("MKD", "MKD", false);
        table_ISO4217.add_value("MMK", "MMK", false);
        table_ISO4217.add_value("MNT", "MNT", false);
        table_ISO4217.add_value("MOP", "MOP", false);
        table_ISO4217.add_value("MRU", "MRU", false);
        table_ISO4217.add_value("MUR", "MUR", false);
        table_ISO4217.add_value("MVR", "MVR", false);
        table_ISO4217.add_value("MWK", "MWK", false);
        table_ISO4217.add_value("MXN", "MXN", false);
        table_ISO4217.add_value("MXV", "MXV", false);
        table_ISO4217.add_value("MYR", "MYR", false);
        table_ISO4217.add_value("MZN", "MZN", false);
        table_ISO4217.add_value("NAD", "NAD", false);
        table_ISO4217.add_value("NGN", "NGN", false);
        table_ISO4217.add_value("NIO", "NIO", false);
        table_ISO4217.add_value("NOK", "NOK", false);
        table_ISO4217.add_value("NPR", "NPR", false);
        table_ISO4217.add_value("NZD", "NZD", false);
        table_ISO4217.add_value("OMR", "OMR", false);
        table_ISO4217.add_value("PAB", "PAB", false);
        table_ISO4217.add_value("PEN", "PEN", false);
        table_ISO4217.add_value("PGK", "PGK", false);
        table_ISO4217.add_value("PHP", "PHP", false);
        table_ISO4217.add_value("PKR", "PKR", false);
        table_ISO4217.add_value("PLN", "PLN", false);
        table_ISO4217.add_value("PYG", "PYG", false);
        table_ISO4217.add_value("QAR", "QAR", false);
        table_ISO4217.add_value("RON", "RON", false);
        table_ISO4217.add_value("RSD", "RSD", false);
        table_ISO4217.add_value("RUB", "RUB", false);
        table_ISO4217.add_value("RWF", "RWF", false);
        table_ISO4217.add_value("SAR", "SAR", false);
        table_ISO4217.add_value("SBD", "SBD", false);
        table_ISO4217.add_value("SCR", "SCR", false);
        table_ISO4217.add_value("SDG", "SDG", false);
        table_ISO4217.add_value("SEK", "SEK", false);
        table_ISO4217.add_value("SGD", "SGD", false);
        table_ISO4217.add_value("SHP", "SHP", false);
        table_ISO4217.add_value("SLE", "SLE", false);
        table_ISO4217.add_value("SOS", "SOS", false);
        table_ISO4217.add_value("SRD", "SRD", false);
        table_ISO4217.add_value("SSP", "SSP", false);
        table_ISO4217.add_value("STN", "STN", false);
        table_ISO4217.add_value("SVC", "SVC", false);
        table_ISO4217.add_value("SYP", "SYP", false);
        table_ISO4217.add_value("SZL", "SZL", false);
        table_ISO4217.add_value("THB", "THB", false);
        table_ISO4217.add_value("TJS", "TJS", false);
        table_ISO4217.add_value("TMT", "TMT", false);
        table_ISO4217.add_value("TND", "TND", false);
        table_ISO4217.add_value("TOP", "TOP", false);
        table_ISO4217.add_value("TRY", "TRY", false);
        table_ISO4217.add_value("TTD", "TTD", false);
        table_ISO4217.add_value("TWD", "TWD", false);
        table_ISO4217.add_value("TZS", "TZS", false);
        table_ISO4217.add_value("UAH", "UAH", false);
        table_ISO4217.add_value("UGX", "UGX", false);
        table_ISO4217.add_value("USD", "USD", false);
        table_ISO4217.add_value("USN", "USN", false);
        table_ISO4217.add_value("UYU", "UYU", false);
        table_ISO4217.add_value("UZS", "UZS", false);
        table_ISO4217.add_value("VED", "VED", false);
        table_ISO4217.add_value("VES", "VES", false);
        table_ISO4217.add_value("VND", "VND", false);
        table_ISO4217.add_value("VUV", "VUV", false);
        table_ISO4217.add_value("WST", "WST", false);
        table_ISO4217.add_value("XAF", "XAF", false);
        table_ISO4217.add_value("XAG", "XAG", false);
        table_ISO4217.add_value("XAU", "XAU", false);
        table_ISO4217.add_value("XBA", "XBA", false);
        table_ISO4217.add_value("XBB", "XBB", false);
        table_ISO4217.add_value("XBC", "XBC", false);
        table_ISO4217.add_value("XBD", "XBD", false);
        table_ISO4217.add_value("XCD", "XCD", false);
        table_ISO4217.add_value("XDR", "XDR", false);
        table_ISO4217.add_value("XOF", "XOF", false);
        table_ISO4217.add_value("XPD", "XPD", false);
        table_ISO4217.add_value("XPF", "XPF", false);
        table_ISO4217.add_value("XPT", "XPT", false);
        table_ISO4217.add_value("XSU", "XSU", false);
        table_ISO4217.add_value("XUA", "XUA", false);
        table_ISO4217.add_value("YER", "YER", false);
        table_ISO4217.add_value("ZAR", "ZAR", false);
        table_ISO4217.add_value("ZMW", "ZMW", false);
        table_ISO4217.add_value("ZWL", "ZWL", false);
        self.add_table(table_ISO4217);
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

    #[test]
    fn test_event_and_message_type_tables() {
        let registry = TableRegistry::new();
        assert!(registry.validate("0003", "A01").is_valid());
        assert!(registry.validate("0003", "V04").is_valid());
        assert!(registry.validate("0003", "O09").is_valid());
        assert!(!registry.validate("0003", "ZZZ").is_valid());
        assert!(registry.validate("0076", "ADT").is_valid());
        assert!(registry.validate("0076", "VXU").is_valid());
        assert!(!registry.validate("0076", "XXX").is_valid());
        assert!(registry.validate("0354", "ADT_A01").is_valid());
        assert!(!registry.validate("0354", "NOPE_NOPE").is_valid());
    }

    #[test]
    fn test_yes_no_and_discharge_tables() {
        let registry = TableRegistry::new();
        assert!(registry.validate("0136", "Y").is_valid());
        assert!(registry.validate("0136", "N").is_valid());
        assert!(!registry.validate("0136", "yes").is_valid());
        // Discharge disposition, including range-expanded codes.
        assert!(registry.validate("0112", "01").is_valid());
        assert!(registry.validate("0112", "20").is_valid());
        assert!(registry.validate("0112", "15").is_valid());
        assert!(registry.validate("0112", "25").is_valid());
        assert!(registry.validate("0112", "35").is_valid());
        assert!(!registry.validate("0112", "99").is_valid());
    }

    #[test]
    fn test_coding_system_and_identifier_tables() {
        let registry = TableRegistry::new();
        assert!(registry.validate("0396", "LN").is_valid());
        assert!(registry.validate("0396", "CVX").is_valid());
        assert!(!registry.validate("0396", "NOPE").is_valid());
        // Identifier type, including the national-person pattern.
        assert!(registry.validate("0203", "MR").is_valid());
        assert!(registry.validate("0203", "NNUSA").is_valid());
        assert!(registry.validate("0203", "NNFRA").is_valid());
        assert!(!registry.validate("0203", "NNx").is_valid());
        assert!(!registry.validate("0203", "ZZ").is_valid());
    }

    #[test]
    fn test_coding_system_patterns() {
        let registry = TableRegistry::new();
        assert!(registry.validate("0396", "HL70001").is_valid());
        assert!(registry.validate("0396", "99LAB").is_valid());
        assert!(registry.validate("0396", "ISO3166").is_valid());
        assert!(!registry.validate("0396", "HL7ABCD").is_valid());
        assert!(!registry.validate("0396", "9").is_valid());
    }

    #[test]
    fn test_priority_min_value_patterns() {
        let registry = TableRegistry::new();
        assert!(registry.validate("0359", "0").is_valid());
        assert!(registry.validate("0359", "1").is_valid());
        assert!(registry.validate("0359", "2").is_valid());
        assert!(registry.validate("0359", "7").is_valid());
        assert!(!registry.validate("0359", "high").is_valid());
        assert!(registry.validate("0418", "5").is_valid());
        assert!(registry.validate("0418", "").is_valid());
    }

    #[test]
    fn test_character_set_tables_keep_spaced_codes() {
        let registry = TableRegistry::new();
        assert!(registry.validate("0211", "UNICODE UTF-8").is_valid());
        assert!(registry.validate("0211", "ASCII").is_valid());
        assert!(!registry.validate("0211", "EBCDIC").is_valid());
        assert!(registry.validate("0356", "2.3").is_valid());
        assert!(registry.validate("0356", "ISO 2022-1994").is_valid());
    }

    #[test]
    fn test_misc_common_tables() {
        let registry = TableRegistry::new();
        assert!(registry.validate("0005", "2106-3").is_valid());
        assert!(!registry.validate("0005", "Martian").is_valid());
        assert!(registry.validate("0200", "L").is_valid());
        assert!(registry.validate("0200", "MSK").is_valid());
        assert!(registry.validate("0190", "H").is_valid());
        assert!(registry.validate("0155", "AL").is_valid());
        assert!(registry.validate("0085", "F").is_valid());
        assert!(registry.validate("4000", "A").is_valid());
        assert!(registry.validate("4000", "I").is_valid());
        assert!(registry.validate("4000", "P").is_valid());
        assert!(!registry.validate("4000", "X").is_valid());
        assert!(registry.validate("ISO4217", "USD").is_valid());
        assert!(registry.validate("ISO4217", "EUR").is_valid());
        assert!(registry.validate("ISO4217", "CNY").is_valid());
        assert!(!registry.validate("ISO4217", "USX").is_valid());
    }

    #[test]
    fn test_pattern_matcher_unit_cases() {
        assert!(pattern_matches("NNxxx", "NNUSA"));
        assert!(!pattern_matches("NNxxx", "NNUS"));
        assert!(!pattern_matches("NNxxx", "NNUSAA"));
        assert!(pattern_matches("HL7nnnn", "HL70001"));
        assert!(!pattern_matches("HL7nnnn", "HL7001"));
        assert!(pattern_matches("Q<integer>D", "Q6D"));
        assert!(pattern_matches("Q<integer>D", "Q12D"));
        assert!(!pattern_matches("Q<integer>D", "QD"));
        assert!(pattern_matches("TD<integer>", "TD1440"));
        assert!(pattern_matches("xID", "BID"));
        assert!(pattern_matches("X12Dennnn", "X12De1234"));
        assert!(!pattern_matches("X12Dennnn", "Y12De1234"));
        assert!(pattern_matches(">=2", "2"));
        assert!(pattern_matches(">=2", "10"));
        assert!(!pattern_matches(">=2", "1"));
        assert!(!pattern_matches(">=2", "2x"));
        assert!(!pattern_matches("NOnnnn", "NOabcd"));
    }

    /// Tables referenced by schemas that are user/site-defined (no HL7-fixed
    /// values) and therefore intentionally left unloaded.
    const USER_TABLES: &[&str] = &[
        "0010", "0018", "0019", "0021", "0022", "0024", "0032", "0033", "0042", "0044", "0045",
        "0046", "0049", "0050", "0051", "0053", "0055", "0056", "0059", "0060", "0064", "0068",
        "0072", "0073", "0079", "0084", "0086", "0087", "0089", "0090", "0099", "0110", "0111",
        "0113", "0114", "0115", "0117", "0118", "0129", "0132", "0139", "0143", "0151", "0152",
        "0171", "0172", "0176", "0182", "0188", "0192", "0212", "0218", "0219", "0222", "0233",
        "0285", "0288", "0289", "0295", "0296", "0297", "0300", "0302", "0303", "0304", "0306",
        "0307", "0308", "0310", "0312", "0313", "0319", "0320", "0327", "0328", "0341", "0342",
        "0343", "0345", "0346", "0361", "0362", "0363", "0378", "0379", "0380", "0381", "0382",
        "0385", "0446", "0447", "0448", "0449", "0476", "0519", "0533", "0541", "0542", "0543",
        "0549", "0552", "0836", "0838", "0865", "0915", "0934",
    ];

    /// Every table referenced by a validation schema must be loaded or be a
    /// known user table; otherwise fields silently skip vocabulary checks.
    #[test]
    fn test_all_schema_tables_are_covered() {
        let registry = TableRegistry::new();
        let schema_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/schemas");
        let mut missing = Vec::new();
        let mut checked = 0;
        for version in std::fs::read_dir(schema_dir).unwrap() {
            let version = version.unwrap().path();
            if !version.is_dir() {
                continue;
            }
            for schema in std::fs::read_dir(&version).unwrap() {
                let path = schema.unwrap().path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let text = std::fs::read_to_string(&path).unwrap();
                let json: serde_json::Value = serde_json::from_str(&text).unwrap();
                for table_id in collect_table_ids(&json) {
                    checked += 1;
                    if registry.get_table(&table_id).is_none()
                        && !USER_TABLES.contains(&table_id.as_str())
                        && !missing.contains(&table_id)
                    {
                        missing.push(table_id);
                    }
                }
            }
        }
        assert!(
            checked > 1000,
            "expected to check table refs, got {checked}"
        );
        assert!(
            missing.is_empty(),
            "schema tables without registry values: {missing:?}"
        );
    }

    fn collect_table_ids(json: &serde_json::Value) -> Vec<String> {
        let mut ids = Vec::new();
        fn walk(value: &serde_json::Value, ids: &mut Vec<String>) {
            match value {
                serde_json::Value::Object(map) => {
                    if let Some(serde_json::Value::String(id)) = map.get("table_id") {
                        ids.push(id.clone());
                    }
                    for v in map.values() {
                        walk(v, ids);
                    }
                }
                serde_json::Value::Array(items) => {
                    for v in items {
                        walk(v, ids);
                    }
                }
                _ => {}
            }
        }
        walk(json, &mut ids);
        ids
    }
}
