//! Performance optimizations for HL7 parsing
//!
//! Pre-allocation and fast-path strategies are now integrated into the main
//! parser functions in `lib.rs`. This module retains tests to verify the
//! optimized parsing paths produce correct results.

#[cfg(test)]
mod tests {
    use crate::parse_field;
    use vyna_hl7_core::delimiters::Delimiters;

    #[test]
    fn test_parse_field_simple() {
        let delims = Delimiters::default();
        let field = parse_field("TEST", &delims).unwrap();
        assert_eq!(field.value(), Some("TEST"));
    }

    #[test]
    fn test_parse_field_components() {
        let delims = Delimiters::default();
        let field = parse_field("DOE^JOHN^A", &delims).unwrap();
        let rep = field.get_repetition(0).unwrap();
        assert_eq!(rep.get_component(0).unwrap().value(), Some("DOE"));
        assert_eq!(rep.get_component(1).unwrap().value(), Some("JOHN"));
        assert_eq!(rep.get_component(2).unwrap().value(), Some("A"));
    }

    #[test]
    fn test_parse_field_repetitions() {
        let delims = Delimiters::default();
        let field = parse_field("Val1~Val2~Val3", &delims).unwrap();
        assert_eq!(field.repetitions.len(), 3);
        assert_eq!(field.get_repetition(0).unwrap().value(), Some("Val1"));
        assert_eq!(field.get_repetition(1).unwrap().value(), Some("Val2"));
        assert_eq!(field.get_repetition(2).unwrap().value(), Some("Val3"));
    }

    #[test]
    fn test_parse_field_subcomponents() {
        let delims = Delimiters::default();
        let field = parse_field("ID1&Auth^ID2", &delims).unwrap();
        let rep = field.get_repetition(0).unwrap();
        let comp0 = rep.get_component(0).unwrap();
        assert_eq!(comp0.get_subcomponent(0).unwrap().as_str(), "ID1");
        assert_eq!(comp0.get_subcomponent(1).unwrap().as_str(), "Auth");
    }

    #[test]
    fn test_parse_field_empty() {
        let delims = Delimiters::default();
        let field = parse_field("", &delims).unwrap();
        assert_eq!(field.value(), Some(""));
        assert_eq!(field.repetitions.len(), 1);
    }

    #[test]
    fn test_parse_field_escape_sequences() {
        let delims = Delimiters::default();
        let field = parse_field("Test\\F\\Value", &delims).unwrap();
        assert_eq!(field.value(), Some("Test|Value"));
    }

    #[test]
    fn test_parse_field_single_subcomponent() {
        // Tests fast path: no subcomponent separator present
        let delims = Delimiters::default();
        let field = parse_field("SimpleValue", &delims).unwrap();
        let comp = field.get_repetition(0).unwrap().get_component(0).unwrap();
        assert_eq!(comp.subcomponents.len(), 1);
        assert_eq!(comp.value(), Some("SimpleValue"));
    }

    #[test]
    fn test_parse_component_preallocation() {
        // Tests that pre-allocation works correctly for multi-component fields
        let delims = Delimiters::default();
        let field = parse_field("A^B^C^D^E", &delims).unwrap();
        let rep = field.get_repetition(0).unwrap();
        assert_eq!(rep.components.len(), 5);
        assert_eq!(rep.get_component(4).unwrap().value(), Some("E"));
    }

    #[test]
    fn test_parse_repetition_preallocation() {
        // Tests that pre-allocation works correctly for multi-repetition fields
        let delims = Delimiters::default();
        let field = parse_field("R1~R2~R3~R4", &delims).unwrap();
        assert_eq!(field.repetitions.len(), 4);
        assert_eq!(field.get_repetition(3).unwrap().value(), Some("R4"));
    }
}
