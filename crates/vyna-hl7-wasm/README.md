# VynaHL7-wasm - HL7 v2.x WebAssembly Bindings

WebAssembly bindings for the VynaHL7 HL7 v2.x library, enabling HL7 message parsing and manipulation in JavaScript and TypeScript environments (browser and Node.js).

## Features

- ✅ **Zero-copy parsing** - Fast message parsing compiled to WebAssembly
- ✅ **Terser API** - Easy field access using path notation (e.g., `PID-5-1`)
- ✅ **Message validation** - Validate against HL7 standards
- ✅ **Type-safe** - Full TypeScript type definitions included
- ✅ **Cross-platform** - Works in browsers and Node.js
- ✅ **Lightweight** - Small bundle size with tree-shaking support

## Installation

### Using npm

```bash
npm install vyna-hl7-wasm
```

### Using yarn

```bash
yarn add vyna-hl7-wasm
```

### Using pnpm

```bash
pnpm add vyna-hl7-wasm
```

## Quick Start

### Browser (ES Modules)

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>VynaHL7 WASM Example</title>
</head>
<body>
    <script type="module">
        import init, { parseMessage, getTerserValue } from './node_modules/vyna-hl7-wasm/pkg/vyna_hl7_wasm.js';

        async function main() {
            // Initialize the WebAssembly module
            await init();

            // Parse an HL7 message
            const hl7 = `MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||ADT^A01|12345|P|2.5
PID|1||MRN123||DOE^JOHN^ALLEN||19800101|M|||123 Main St^^Boston^MA^02101||555-1234`;

            const message = parseMessage(hl7);

            // Extract fields using Terser paths
            const patientName = getTerserValue(message, "PID-5");
            const familyName = getTerserValue(message, "PID-5-0");
            const givenName = getTerserValue(message, "PID-5-1");
            const dob = getTerserValue(message, "PID-7");

            console.log("Patient:", familyName, givenName);
            console.log("DOB:", dob);
        }

        main();
    </script>
</body>
</html>
```

### Node.js

```typescript
import init, { parseMessage, extractPatientDemographics } from 'vyna-hl7-wasm';

async function main() {
    // Initialize WASM module
    await init();

    const hl7 = `MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5
PID|1||MRN123||DOE^JOHN||19800101|M`;

    const message = parseMessage(hl7);

    // Extract patient demographics
    const demographics = extractPatientDemographics(message);
    console.log(demographics);
    // {
    //   mrn: "MRN123",
    //   family_name: "DOE",
    //   given_name: "JOHN",
    //   date_of_birth: "19800101",
    //   gender: "M"
    // }
}

main();
```

### TypeScript

```typescript
import init, {
    parseMessage,
    getTerserValue,
    setTerserValue,
    validateMessage,
    WasmMessage
} from 'vyna-hl7-wasm';

async function processHL7(hl7String: string): Promise<void> {
    await init();

    // Parse message
    const message: WasmMessage = parseMessage(hl7String);

    // Read fields
    const messageType = message.messageType();
    const sendingApp = message.sendingApplication();
    console.log(`Processing ${messageType} from ${sendingApp}`);

    // Modify fields
    setTerserValue(message, "PID-5-0", "SMITH");

    // Validate
    const result = validateMessage(message);
    if (!result.isValid()) {
        const errors = result.toJson().errors;
        console.error("Validation errors:", errors);
    }

    // Encode back to HL7
    const encoded = message.encode();
    console.log(encoded);
}
```

## API Reference

### Message Parsing

#### `parseMessage(input: string): WasmMessage`

Parse an HL7 message string into a message object.

```typescript
const message = parseMessage(hl7String);
```

#### `createMessage(version: string, messageType: string, sendingApp: string, sendingFac: string): WasmMessage`

Create a new HL7 message with MSH segment.

```typescript
const message = createMessage("2.5", "ADT^A01", "MyApp", "MyFacility");
```

### Field Access (Terser API)

#### `getTerserValue(message: WasmMessage, path: string): string | undefined`

Get a single field value using Terser path notation.

```typescript
const name = getTerserValue(message, "PID-5");
const family = getTerserValue(message, "PID-5-0");  // First component
const given = getTerserValue(message, "PID-5-1");   // Second component
```

#### `setTerserValue(message: WasmMessage, path: string, value: string): void`

Set a field value.

```typescript
setTerserValue(message, "PID-5-0", "SMITH");
setTerserValue(message, "PID-8", "F");
```

#### `getTerserValues(message: WasmMessage, paths: string[]): Record<string, string | null>`

Get multiple values at once.

```typescript
const values = getTerserValues(message, [
    "PID-5-0",
    "PID-5-1",
    "PID-7",
    "PID-8"
]);
```

### Validation

#### `validateMessage(message: WasmMessage): WasmValidationResult`

Validate a message against HL7 standards.

```typescript
const result = validateMessage(message);

if (result.isValid()) {
    console.log("Message is valid!");
} else {
    const json = result.toJson();
    console.log(`Errors: ${result.errorCount()}`);
    console.log(`Warnings: ${result.warningCount()}`);
    console.log(json.errors);
}
```

### Helper Functions

#### `extractPatientDemographics(message: WasmMessage): PatientDemographics`

Extract common patient fields from ADT messages.

```typescript
const demographics = extractPatientDemographics(message);
console.log(demographics.family_name, demographics.given_name);
```

#### `extractObservations(message: WasmMessage): Observation[]`

Extract observations from ORU messages.

```typescript
const observations = extractObservations(message);
observations.forEach(obs => {
    console.log(`${obs.test_name}: ${obs.value} ${obs.units}`);
});
```

### Message Methods

```typescript
const message: WasmMessage = parseMessage(hl7);

// Message metadata
message.version();              // "2.5"
message.messageType();          // "ADT^A01"
message.sendingApplication();   // "SendApp"
message.sendingFacility();      // "SendFac"
message.controlId();            // "12345"

// Structure
message.segmentCount();         // 3
message.segmentIds();           // ["MSH", "PID", "PV1"]

// Conversion
message.encode();               // HL7 string
message.toJson();               // JSON representation
```

## Terser Path Notation

The Terser API uses simple path notation to access fields:

| Path | Description |
|------|-------------|
| `PID-5` | PID segment, field 5 (full field) |
| `PID-5-1` | PID segment, field 5, component 1 (1-indexed) |
| `PID-5-2` | PID segment, field 5, component 2 |
| `OBX(2)-5` | Second OBX segment (1-indexed), field 5 |
| `PID-11(1)-1` | PID segment, field 11, first repetition, component 1 |

## Building from Source

### Prerequisites

- Rust (latest stable)
- wasm-pack (`cargo install wasm-pack`)
- Node.js (for testing)

### Build for Web

```bash
cd crates/vyna-hl7-wasm
wasm-pack build --target web
```

### Build for Node.js

```bash
wasm-pack build --target nodejs
```

### Build for Bundlers (webpack, etc.)

```bash
wasm-pack build --target bundler
```

### Run Tests

```bash
wasm-pack test --headless --firefox
```

## Performance

VynaHL7-WASM benefits from Rust's performance and WebAssembly's near-native speed:

- **Parsing**: ~2-5 µs for small messages (3 segments)
- **Parsing**: ~8-12 µs for medium messages (8 segments)
- **Terser access**: ~500-800 ns per field access
- **Bundle size**: ~200-300 KB (minified + gzip)

## Examples

See the `examples/` directory for complete working examples:

- `browser.html` - Browser usage with vanilla JavaScript
- `node-example.js` - Node.js usage
- `typescript-example.ts` - TypeScript usage
- `react-example.tsx` - React component example

## Browser Compatibility

- Chrome/Edge 91+
- Firefox 89+
- Safari 15+
- Node.js 16+

Requires WebAssembly support (available in all modern browsers).

## License

Copyright (c) 2024-2026 Alex SHAO Fan <alex.shaofan@flostel.com>. All rights reserved.

This software is proprietary and confidential. Unauthorized copying, distribution, or use is strictly prohibited.

## Contributing

This crate is developed in the private [VynaHL7 repository](https://gitlab.flostel.com/alexshao/vyna-hl7). Internal contributors: please create a branch and submit a merge request.

## Resources

- [VynaHL7 Main Repository](https://gitlab.flostel.com/alexshao/vyna-hl7)
- [HL7 v2.x Documentation](https://www.hl7.org/implement/standards/product_brief.cfm?product_id=185)
- [WebAssembly Documentation](https://webassembly.org/)
