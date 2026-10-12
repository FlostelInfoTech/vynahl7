# Terser Component Indexing Issue

## Problem

The Terser API uses **0-based indexing** for components, which differs from HL7 standard 1-based numbering.

### Example

For the field `DOE^JOHN^A` (HL7 PID-5 Patient Name):

| HL7 Standard | Terser Path | Returns |
|--------------|-------------|---------|
| PID-5-1 (Family Name) | `PID-5` or `PID-5-0` | "DOE" |
| PID-5-2 (Given Name) | `PID-5-1` | "JOHN" |
| PID-5-3 (Middle Name) | `PID-5-2` | "A" |

## Fix Status

✅ **ALL FIXES COMPLETE** - All component indexing issues have been resolved.

### Patient Converter (patient.rs) - ✅ FIXED

- **convert_identifiers**: ✅ FIXED
  - PID-3-1 (ID) → uses `PID-3` (0-based indexing)
  - PID-3-4 (Authority) → uses `PID-3-3`
  - PID-3-5 (Type Code) → uses `PID-3-4`

- **convert_names**: ✅ FIXED
  - PID-5-1 (Family) → uses `PID-5`
  - PID-5-2 (Given) → uses `PID-5-1`
  - PID-5-3 (Middle) → uses `PID-5-2`
  - PID-5-4 (Suffix) → uses `PID-5-3`
  - PID-5-5 (Prefix) → uses `PID-5-4`
  - PID-5-7 (Type) → uses `PID-5-6`

- **convert_addresses**: ✅ FIXED
  - PID-11-1 (Street) → uses `PID-11`
  - PID-11-2 (Other) → uses `PID-11-1`
  - PID-11-3 (City) → uses `PID-11-2`
  - PID-11-4 (State) → uses `PID-11-3`
  - PID-11-5 (Postal) → uses `PID-11-4`
  - PID-11-6 (Country) → uses `PID-11-5`
  - PID-11-7 (Type) → uses `PID-11-6`

### Observation Converter (observation.rs) - ✅ FIXED

- **convert_single**: ✅ FIXED
  - OBX-3-1 (Code ID) → uses `OBX-3`
  - OBX-3-2 (Text) → uses `OBX-3-1`
  - OBX-3-3 (System) → uses `OBX-3-2`
  - OBX-16-1 (Observer ID) → uses `OBX-16`
  - OBX-16-2 (Observer Name) → uses `OBX-16-1`

- **set_observation_value**: ✅ FIXED
  - Value component 1 → uses value_path
  - Value component 2 → uses value_path + "-1"
  - Value component 3 → uses value_path + "-2"

### Practitioner Converter (practitioner.rs) - ✅ FIXED

- **convert_xcn_to_practitioner**: ✅ FIXED
  - XCN-1 (ID) → uses base_path
  - XCN-2 (Family) → uses base_path + "-1"
  - XCN-3 (Given) → uses base_path + "-2"
  - XCN-4 (Middle) → uses base_path + "-3"
  - XCN-5 (Suffix) → uses base_path + "-4"
  - XCN-6 (Prefix) → uses base_path + "-5"
  - XCN-7 (Degree) → uses base_path + "-6"
  - XCN-9 (Authority) → uses base_path + "-8"
  - XCN-13 (ID Type) → uses base_path + "-12"

## Testing

✅ All tests passing:
```bash
cargo test -p vyna-hl7-fhir --lib
```

Result: **8 tests passed, 0 failed**
