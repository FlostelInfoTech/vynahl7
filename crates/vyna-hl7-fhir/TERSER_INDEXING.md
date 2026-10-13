# Terser Path Indexing (1-Based)

Terser paths use **1-based indexing throughout**, matching the HL7
standard and HAPI conventions:

- Segments: `OBX(1)` is the first OBX; `OBX(0)` is invalid.
- Repetitions: `PID-5(1)` is the first repetition; `(0)` is invalid.
  Omitting the index reads the first repetition.
- Components: `PID-5-1` is the first component; `-0` is invalid.
  A whole-field read (`PID-5`) returns the first component.

## Example

For the field `DOE^JOHN^A` (HL7 PID-5 Patient Name):

| HL7 Standard | Terser Path | Returns |
|--------------|-------------|---------|
| PID-5-1 (Family Name) | `PID-5` or `PID-5-1` | "DOE" |
| PID-5-2 (Given Name) | `PID-5-2` | "JOHN" |
| PID-5-3 (Middle Name) | `PID-5-3` | "A" |

## History

An earlier revision of this document claimed 0-based component
indexing and listed converter mappings to match. That was wrong: the
Terser implementation has always been 1-based
(`test_1_based_repetition_indexing`, whole-field reads returning
component 1), and the converters were fixed to use 1-based paths
throughout (see `test_identifiers_read_every_repetition_with_authority_and_type`
and friends). Only the internal `Vec` storage is 0-based.
