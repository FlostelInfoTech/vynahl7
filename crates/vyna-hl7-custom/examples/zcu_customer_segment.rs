//! Example: ZCU - Customer/Financial Information Segment with Validation
//!
//! This example demonstrates a custom Z-segment with business rule validation.
//! The ZCU segment contains customer financial information and ensures that
//! balances cannot be negative.

use vyna_hl7_custom::{z_segment, CustomSegment, CustomSegmentError, MessageExt};
use vyna_hl7_parser::parse_message;

// Define the ZCU custom segment with validation
z_segment! {
    ZCU,
    id = "ZCU",
    fields = {
        1 => customer_id: String,
        2 => account_number: String,
        3 => balance: Option<f64>,
        4 => credit_limit: Option<f64>,
        5 => account_status: Option<String>, // ACTIVE, SUSPENDED, CLOSED
    },
    validate = |s: &ZCU| {
        // Validate balance is not negative
        if let Some(balance) = s.balance
            && balance < 0.0 {
                return Err(CustomSegmentError::validation_failed(
                    "ZCU",
                    "Account balance cannot be negative"
                ));
            }

        // Validate credit limit is not negative
        if let Some(credit_limit) = s.credit_limit
            && credit_limit < 0.0 {
                return Err(CustomSegmentError::validation_failed(
                    "ZCU",
                    "Credit limit cannot be negative"
                ));
            }

        // Validate account status is valid
        if let Some(ref status) = s.account_status {
            match status.as_str() {
                "ACTIVE" | "SUSPENDED" | "CLOSED" => Ok(()),
                _ => Err(CustomSegmentError::validation_failed(
                    "ZCU",
                    format!("Invalid account status: {}", status)
                )),
            }
        } else {
            Ok(())
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Valid ZCU Segment ===");

    // Create a valid ZCU segment
    let zcu = ZCU::builder()
        .customer_id("CUST001")
        .account_number("ACC-2024-001")
        .balance(1500.75)
        .credit_limit(5000.0)
        .account_status("ACTIVE")
        .build()?;

    println!("Customer ID: {}", zcu.customer_id);
    println!("Account: {}", zcu.account_number);
    println!("Balance: ${:.2}", zcu.balance.unwrap_or(0.0));
    println!("Credit Limit: ${:.2}", zcu.credit_limit.unwrap_or(0.0));
    println!("Status: {}", zcu.account_status.as_deref().unwrap_or("N/A"));

    println!("\n=== Invalid ZCU Segment (Negative Balance) ===");

    // Try to create an invalid segment with negative balance
    let invalid_zcu = ZCU::builder()
        .customer_id("CUST002")
        .account_number("ACC-2024-002")
        .balance(-100.0) // This will fail validation
        .build();

    match invalid_zcu {
        Ok(_) => println!("ERROR: Validation should have failed!"),
        Err(e) => println!("Validation correctly failed: {}", e),
    }

    println!("\n=== Invalid ZCU Segment (Invalid Status) ===");

    // Try to create an invalid segment with invalid status
    let invalid_status = ZCU::builder()
        .customer_id("CUST003")
        .account_number("ACC-2024-003")
        .balance(100.0)
        .account_status("INVALID") // This will fail validation
        .build();

    match invalid_status {
        Ok(_) => println!("ERROR: Validation should have failed!"),
        Err(e) => println!("Validation correctly failed: {}", e),
    }

    println!("\n=== Parsing ZCU from Message ===");

    // Parse a message with ZCU
    let hl7_message = "MSH|^~\\&|BillingApp|Hospital|Finance|Corp|20240315||DFT^P03|MSG002|P|2.5\r\
                       PID|1|54321|98765^^^MRN|SMITH^JANE^B||19750315|F\r\
                       FT1|1||20240315|CONSULT|CONSULTATION|||1|150.00\r\
                       ZCU|CUST001|ACC-2024-001|1500.75|5000.00|ACTIVE";

    let message = parse_message(hl7_message)?;

    if let Some(zcu) = message.get_custom_segment::<ZCU>()? {
        println!("Extracted ZCU from message:");
        println!("  Customer: {}", zcu.customer_id);
        println!("  Balance: ${:.2}", zcu.balance.unwrap_or(0.0));
        println!("  Status: {}", zcu.account_status.as_deref().unwrap_or("N/A"));
    }

    Ok(())
}
