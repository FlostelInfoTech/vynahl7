/**
 * TypeScript type definitions for VynaHL7 WebAssembly bindings
 *
 * VynaHL7 is a comprehensive HL7 v2.x library for parsing, validating,
 * and manipulating healthcare messages in JavaScript/TypeScript.
 */

/**
 * Initialize the WebAssembly module (web/bundler targets)
 * Must be awaited before using any other functions.
 *
 * NOTE: the Rust `#[wasm_bindgen(start)]` hook is additionally exported as a
 * synchronous named `init(): void` which runs automatically on instantiation.
 * It does not load the module, so never `await` it for initialization —
 * always await this default-export loader instead. On the nodejs target the
 * module loads synchronously at require time and no call is needed at all.
 *
 * @example
 * ```typescript
 * import init, { parseMessage } from 'vyna-hl7-wasm';
 * await init();
 * const message = parseMessage(hl7);
 * ```
 */
export default function init(
  module_or_path?: InitInput | Promise<InitInput>
): Promise<InitOutput>;

/** Loader input accepted by the default `init` (mirrors wasm-bindgen). */
export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

/** Instantiated wasm module exports (mirrors wasm-bindgen). */
export interface InitOutput {
  readonly memory: WebAssembly.Memory;
}

/**
 * HL7 Message wrapper
 */
export class WasmMessage {
  /** Get the HL7 version (e.g., "2.5") */
  version(): string | undefined;

  /** Get the message type (e.g., "ADT^A01") */
  messageType(): string | undefined;

  /** Get the sending application */
  sendingApplication(): string | undefined;

  /** Get the sending facility */
  sendingFacility(): string | undefined;

  /** Get the receiving application */
  receivingApplication(): string | undefined;

  /** Get the receiving facility */
  receivingFacility(): string | undefined;

  /** Get the message control ID */
  controlId(): string | undefined;

  /** Encode the message back to HL7 string format */
  encode(): string;

  /** Get the number of segments in the message */
  segmentCount(): number;

  /** Get an array of segment IDs */
  segmentIds(): string[];

  /** Convert message to JSON representation */
  toJson(): MessageJson;
}

/**
 * Validation result
 */
export class WasmValidationResult {
  /** Check if the message is valid */
  isValid(): boolean;

  /** Get the number of validation errors */
  errorCount(): number;

  /** Get the number of validation warnings */
  warningCount(): number;

  /** Convert validation result to JSON */
  toJson(): ValidationResultJson;
}

/**
 * Parse an HL7 message string
 *
 * @param input - The HL7 message string
 * @returns The parsed message
 * @throws Error if parsing fails
 *
 * @example
 * ```typescript
 * const message = parseMessage(
 *   "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\n" +
 *   "PID|1||MRN123||DOE^JOHN||19800101|M"
 * );
 * ```
 */
export function parseMessage(input: string): WasmMessage;

/**
 * Get the HL7 version from a message
 *
 * @param message - The parsed message
 * @returns The HL7 version string (e.g., "2.5")
 */
export function getVersion(message: WasmMessage): string | undefined;

/**
 * Get the message type from a message
 *
 * @param message - The parsed message
 * @returns The message type string (e.g., "ADT^A01")
 */
export function getMessageType(message: WasmMessage): string | undefined;

/**
 * Encode a message back to HL7 string format
 *
 * @param message - The message to encode
 * @returns The HL7 string
 */
export function encodeMessage(message: WasmMessage): string;

/**
 * Get a field value using Terser path notation
 *
 * @param message - The parsed message
 * @param path - The Terser path (e.g., "PID-5-1")
 * @returns The field value if found, or undefined
 * @throws Error if the path is malformed
 *
 * @example
 * ```typescript
 * const patientName = getTerserValue(message, "PID-5");
 * const familyName = getTerserValue(message, "PID-5-1");
 * const givenName = getTerserValue(message, "PID-5-2");
 * ```
 */
export function getTerserValue(message: WasmMessage, path: string): string | undefined;

/**
 * Set a field value using Terser path notation
 *
 * @param message - The message to modify
 * @param path - The Terser path (e.g., "PID-5-1")
 * @param value - The value to set
 * @throws Error if the path is malformed
 *
 * @example
 * ```typescript
 * setTerserValue(message, "PID-5-1", "SMITH");
 * setTerserValue(message, "PID-5-2", "JOHN");
 * ```
 */
export function setTerserValue(message: WasmMessage, path: string, value: string): void;

/**
 * Get multiple values at once using Terser paths
 *
 * @param message - The message to query
 * @param paths - Array of Terser paths
 * @returns Object mapping paths to values
 * @throws Error if any path is malformed
 *
 * @example
 * ```typescript
 * const values = getTerserValues(message, [
 *   "PID-5-1",
 *   "PID-5-2",
 *   "PID-7",
 *   "PID-8"
 * ]);
 * // { "PID-5-1": "DOE", "PID-5-2": "JOHN", "PID-7": "19800101", "PID-8": "M" }
 * ```
 */
export function getTerserValues(message: WasmMessage, paths: string[]): Record<string, string | null>;

/**
 * Validate a message against HL7 standards
 *
 * @param message - The message to validate
 * @returns Validation result
 *
 * @example
 * ```typescript
 * const result = validateMessage(message);
 * if (!result.isValid()) {
 *   const errors = result.toJson().errors;
 *   console.error("Validation errors:", errors);
 * }
 * ```
 */
export function validateMessage(message: WasmMessage): WasmValidationResult;

/**
 * Create a new HL7 message with MSH segment
 *
 * @param version - HL7 version (e.g., "2.5")
 * @param messageType - Message type (e.g., "ADT^A01")
 * @param sendingApp - Sending application
 * @param sendingFac - Sending facility
 * @returns A new message with MSH segment
 * @throws Error if an argument is empty, holds HL7 structural
 * characters, or the version is unsupported
 *
 * @example
 * ```typescript
 * const message = createMessage("2.5", "ADT^A01", "MyApp", "MyFacility");
 * ```
 */
export function createMessage(
  version: string,
  messageType: string,
  sendingApp: string,
  sendingFac: string
): WasmMessage;

/**
 * Extract patient demographics from a message
 *
 * @param message - The message (typically ADT)
 * @returns Patient demographics object
 * @throws Error if result serialization fails
 *
 * @example
 * ```typescript
 * const demographics = extractPatientDemographics(message);
 * console.log(demographics.family_name, demographics.given_name);
 * ```
 */
export function extractPatientDemographics(message: WasmMessage): PatientDemographics;

/**
 * Extract observations from an ORU message
 *
 * @param message - The ORU message
 * @returns Array of observation objects
 * @throws Error if result serialization fails
 *
 * @example
 * ```typescript
 * const observations = extractObservations(message);
 * observations.forEach(obs => {
 *   console.log(obs.test_name, obs.value, obs.units);
 * });
 * ```
 */
export function extractObservations(message: WasmMessage): Observation[];

/**
 * List all supported HL7 versions
 *
 * @returns Array of version strings (e.g. `["2.3", "2.4", "2.5"]`)
 */
export function listHL7Versions(): string[];

/**
 * List all message types available for a given HL7 version
 *
 * @param version - HL7 version string (e.g., "2.5")
 * @returns Array of message type descriptors, sorted by type then trigger
 * @throws Error if the version is unsupported
 */
export function listMessageTypes(version: string): MessageTypeInfo[];

/**
 * Get the full message definition (schema) for a message type
 *
 * @param version - HL7 version string (e.g., "2.5")
 * @param messageType - Message type (e.g., "ADT")
 * @param triggerEvent - Trigger event (e.g., "A01"), or "" for types like "ACK"
 * @returns The message schema with all segments and field definitions
 * @throws Error if the version is unsupported or no schema exists
 * for the message type
 */
export function getMessageDefinition(
  version: string,
  messageType: string,
  triggerEvent: string
): MessageSchema;

/**
 * Get the definition of a single segment
 *
 * @param version - HL7 version string (e.g., "2.5")
 * @param segmentId - Segment identifier (e.g., "PID")
 * @returns The segment definition with all field metadata
 * @throws Error if the version is unsupported or the segment is unknown
 */
export function getSegmentDefinition(version: string, segmentId: string): SegmentResult;

/**
 * Get an HL7 vocabulary table with all code values
 *
 * @param tableId - The HL7 table ID (e.g., "0001")
 * @returns The table with code values and descriptions
 * @throws Error if the table ID is unknown
 */
export function getVocabularyTable(tableId: string): VocabularyTable;

/**
 * Search segment and field definitions (case-insensitive, up to 50 matches)
 *
 * @param version - HL7 version string (e.g., "2.5")
 * @param query - Search query; must not be empty
 * @returns Matching segments and fields with match context
 * @throws Error if the version is unsupported or the query is empty
 */
export function searchDefinitions(version: string, query: string): SearchResult[];

// Type definitions for JSON structures

export interface MessageJson {
  version?: string;
  message_type?: string;
  sending_application?: string;
  sending_facility?: string;
  control_id?: string;
  segments: SegmentJson[];
}

export interface SegmentJson {
  id: string;
  fields: string[];
}

export interface ValidationResultJson {
  is_valid: boolean;
  errors: ValidationError[];
  warnings: ValidationError[];
}

export interface ValidationError {
  location: string;
  message: string;
  severity: string;
}

export interface PatientDemographics {
  patient_id?: string;
  mrn?: string;
  family_name?: string;
  given_name?: string;
  date_of_birth?: string;
  gender?: string;
  address?: string;
  phone?: string;
}

export interface Observation {
  set_id?: string;
  value_type?: string;
  identifier?: string;
  test_name?: string;
  value?: string;
  units?: string;
  reference_range?: string;
  abnormal_flags?: string;
  status?: string;
  observation_datetime?: string;
}

export interface MessageTypeInfo {
  messageType: string;
  triggerEvent: string;
  description: string;
}

export interface MessageSchema {
  messageType: string;
  triggerEvent: string;
  version: string;
  description: string;
  segments: Record<string, SegmentDefinition>;
}

export interface SegmentDefinition {
  name: string;
  required: boolean;
  repeating: boolean;
  fields: Record<string, FieldDefinition>;
}

export interface FieldDefinition {
  name: string;
  dataType: string;
  required: boolean;
  repeating: boolean;
  maxLength: number | null;
  tableId?: string;
}

export interface SegmentResult {
  segmentId: string;
  name: string;
  required: boolean;
  repeating: boolean;
  fields: Record<string, FieldDefinition>;
}

export interface SearchResult {
  segmentId: string;
  fieldIndex: number | null;
  name: string;
  matchType: string;
  path: string;
  dataType: string | null;
}

export interface VocabularyTable {
  table_id: string;
  name: string;
  description: string;
  values: Record<string, TableValue>;
  patterns: string[];
}

export interface TableValue {
  code: string;
  description: string;
  deprecated: boolean;
}
