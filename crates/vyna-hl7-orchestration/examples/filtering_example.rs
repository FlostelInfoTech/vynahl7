//! Message filtering example
//!
//! This example demonstrates how to filter HL7 messages using predicates
//! with both ALL (AND) and ANY (OR) filter modes.

use vyna_hl7_core::{Field, Message, Segment};
use vyna_hl7_orchestration::filtering::MessageFilter;
use vyna_hl7_terser::Terser;

fn main() {
    println!("=== Message Filtering Example ===\n");

    // Example 1: ALL mode (all filters must pass)
    println!("1. ALL Mode Filter (Production AND ADT messages only):");
    let mut all_filter = MessageFilter::new(); // Default is ALL mode

    all_filter.add_rule("production_only", |msg| {
        let terser = Terser::new(msg);
        terser.get("MSH-11").ok().flatten() == Some("P")
    });

    all_filter.add_rule("adt_only", |msg| {
        let terser = Terser::new(msg);
        terser
            .get("MSH-9-1")
            .ok()
            .flatten()
            .map(|v| v.contains("ADT"))
            .unwrap_or(false)
    });

    let prod_adt = create_production_adt();
    let test_adt = create_test_adt();
    let prod_oru = create_production_oru();

    println!("   Production ADT: {:?}", all_filter.filter(&prod_adt));
    println!("   Test ADT:       {:?}", all_filter.filter(&test_adt));
    println!("   Production ORU: {:?}", all_filter.filter(&prod_oru));
    println!();

    // Example 2: ANY mode (at least one filter must pass)
    println!("2. ANY Mode Filter (Production OR Test messages):");
    let mut any_filter = MessageFilter::new_any(); // ANY mode

    any_filter.add_rule("production", |msg| {
        let terser = Terser::new(msg);
        terser.get("MSH-11").ok().flatten() == Some("P")
    });

    any_filter.add_rule("test", |msg| {
        let terser = Terser::new(msg);
        terser.get("MSH-11").ok().flatten() == Some("T")
    });

    let debug_msg = create_debug_message();

    println!("   Production ADT: {:?}", any_filter.filter(&prod_adt));
    println!("   Test ADT:       {:?}", any_filter.filter(&test_adt));
    println!("   Debug message:  {:?}", any_filter.filter(&debug_msg));
    println!();

    // Example 3: Complex filter with multiple criteria
    println!("3. Complex Filter (High priority production messages):");
    let mut complex_filter = MessageFilter::new();

    complex_filter.add_rule("production", |msg| {
        let terser = Terser::new(msg);
        terser.get("MSH-11").ok().flatten() == Some("P")
    });

    complex_filter.add_rule("high_priority", |msg| {
        // Check if message has MSH segment (all valid HL7 messages should)
        msg.segments.iter().any(|seg| seg.id == "MSH")
    });

    println!("   Production ADT: {:?}", complex_filter.filter(&prod_adt));
    println!("   Test ADT:       {:?}", complex_filter.filter(&test_adt));
    println!();

    // Example 4: Using matches() for boolean checks
    println!("4. Boolean Filter Checks:");
    let mut simple_filter = MessageFilter::new();
    simple_filter.add_rule("adt_messages", |msg| {
        let terser = Terser::new(msg);
        terser
            .get("MSH-9-1")
            .ok()
            .flatten()
            .map(|v| v.contains("ADT"))
            .unwrap_or(false)
    });

    println!(
        "   Production ADT matches: {}",
        simple_filter.matches(&prod_adt)
    );
    println!(
        "   Production ORU matches: {}",
        simple_filter.matches(&prod_oru)
    );
    println!();

    println!("✅ Filtering examples completed!");
}

fn create_production_adt() -> Message {
    let mut msg = Message::default();
    let mut msh = Segment::new("MSH");
    msh.fields.push(Field::from_value("|"));
    msh.fields.push(Field::from_value("^~\\&"));
    msh.fields.push(Field::from_value("SendingApp"));
    msh.fields.push(Field::from_value("SendingFac"));
    msh.fields.push(Field::from_value("ReceivingApp"));
    msh.fields.push(Field::from_value("ReceivingFac"));
    msh.fields.push(Field::from_value("20231101120000"));
    msh.fields.push(Field::from_value(""));
    msh.fields.push(Field::from_value("ADT^A01"));
    msh.fields.push(Field::from_value("MSG001"));
    msh.fields.push(Field::from_value("P")); // Production
    msg.segments.push(msh);
    msg
}

fn create_test_adt() -> Message {
    let mut msg = Message::default();
    let mut msh = Segment::new("MSH");
    msh.fields.push(Field::from_value("|"));
    msh.fields.push(Field::from_value("^~\\&"));
    msh.fields.push(Field::from_value("SendingApp"));
    msh.fields.push(Field::from_value("SendingFac"));
    msh.fields.push(Field::from_value("ReceivingApp"));
    msh.fields.push(Field::from_value("ReceivingFac"));
    msh.fields.push(Field::from_value("20231101120000"));
    msh.fields.push(Field::from_value(""));
    msh.fields.push(Field::from_value("ADT^A01"));
    msh.fields.push(Field::from_value("MSG002"));
    msh.fields.push(Field::from_value("T")); // Test
    msg.segments.push(msh);
    msg
}

fn create_production_oru() -> Message {
    let mut msg = Message::default();
    let mut msh = Segment::new("MSH");
    msh.fields.push(Field::from_value("|"));
    msh.fields.push(Field::from_value("^~\\&"));
    msh.fields.push(Field::from_value("LabSystem"));
    msh.fields.push(Field::from_value("Hospital"));
    msh.fields.push(Field::from_value("ReceivingApp"));
    msh.fields.push(Field::from_value("ReceivingFac"));
    msh.fields.push(Field::from_value("20231101120000"));
    msh.fields.push(Field::from_value(""));
    msh.fields.push(Field::from_value("ORU^R01"));
    msh.fields.push(Field::from_value("MSG003"));
    msh.fields.push(Field::from_value("P")); // Production
    msg.segments.push(msh);
    msg
}

fn create_debug_message() -> Message {
    let mut msg = Message::default();
    let mut msh = Segment::new("MSH");
    msh.fields.push(Field::from_value("|"));
    msh.fields.push(Field::from_value("^~\\&"));
    msh.fields.push(Field::from_value("System"));
    msh.fields.push(Field::from_value("Hospital"));
    msh.fields.push(Field::from_value("ReceivingApp"));
    msh.fields.push(Field::from_value("ReceivingFac"));
    msh.fields.push(Field::from_value("20231101120000"));
    msh.fields.push(Field::from_value(""));
    msh.fields.push(Field::from_value("ADT^A01"));
    msh.fields.push(Field::from_value("MSG004"));
    msh.fields.push(Field::from_value("D")); // Debug
    msg.segments.push(msh);
    msg
}
