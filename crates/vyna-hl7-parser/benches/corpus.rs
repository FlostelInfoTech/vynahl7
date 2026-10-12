//! Shared realistic HL7 message corpus for benchmarking
//!
//! Contains 9 realistic message constants covering the major HL7 message types
//! used in production healthcare systems, plus 2 generator functions for
//! batch/file messages.

/// Full ADT^A01 with NK1, IN1, AL1, multiple addresses/phones (~600B)
pub const ADT_A01_FULL: &str = "\
MSH|^~\\&|HIS|HOSPITAL|RIS|RADIOLOGY|20240315143000||ADT^A01^ADT_A01|MSG00001|P|2.5|||AL|NE\r\
EVN|A01|20240315143000|||ADMIN^SMITH^LISA\r\
PID|1||MRN12345^^^HOSPITAL^MR~SSN987654321^^^SSA^SS||DOE^JOHN^ALLEN^JR||19800315|M||2106-3^White^CDCREC|123 MAIN ST^^BOSTON^MA^02101^USA^H~456 OAK AVE^^CAMBRIDGE^MA^02139^USA^O||^PRN^PH^^1^617^5551234~^WPN^PH^^1^617^5559999~^NET^Internet^john.doe@example.com|||M||VN00001^^^HOSPITAL^VN|987-65-4321\r\
NK1|1|DOE^JANE^M||SPO^Spouse^HL70063|^PRN^PH^^1^617^5555678||EC^Emergency Contact^HL70131\r\
PV1|1|I|ICU^101^A^HOSPITAL||||1234567^SMITH^JANE^M^MD^^MD|9876543^JONES^BOB^A^MD^^MD||MED||||VIP|||1234567^SMITH^JANE^M^MD^^MD|IP||SELF|||||||||||||||||||HOSPITAL|||||20240315140000\r\
AL1|1|DA|PCN^Penicillin^L|SV^Severe|Anaphylaxis|20200101\r\
IN1|1|BCBS001^Blue Cross^L|9876|BLUE CROSS BLUE SHIELD|PO BOX 1234^^BOSTON^MA^02101|^PRN^PH^^1^800^5551111|GRP12345|||||||DOE^JOHN^A|SEL|19800315|123 MAIN ST^^BOSTON^MA^02101||||||||||||||||POL123456";

/// Comprehensive ORU^R01 with 2 OBR groups, mix of NM/ST/TX OBX, NTE, abnormal flags (~2.5KB)
pub const ORU_R01_COMPREHENSIVE: &str = "\
MSH|^~\\&|LAB|HOSPITAL|EMR|HOSPITAL|20240315150000||ORU^R01^ORU_R01|MSG00002|P|2.5|||AL|NE\r\
PID|1||MRN12345^^^HOSPITAL^MR||DOE^JOHN^ALLEN||19800315|M|||123 MAIN ST^^BOSTON^MA^02101^USA\r\
PV1|1|O|ER^201^B^HOSPITAL||||1234567^SMITH^JANE^M^MD\r\
ORC|RE|ORD100001|LAB200001||CM||||20240315120000|||1234567^SMITH^JANE^M^MD\r\
OBR|1|ORD100001|LAB200001|CBC^Complete Blood Count^LN|||20240315110000|||||||||1234567^SMITH^JANE^M^MD||||||20240315120000|||F\r\
OBX|1|NM|WBC^White Blood Cell Count^LN||12.5|10*9/L|4.5-11.0|HH|||F|||20240315115500\r\
OBX|2|NM|RBC^Red Blood Cell Count^LN||4.8|10*12/L|4.2-5.9|N|||F|||20240315115500\r\
OBX|3|NM|HGB^Hemoglobin^LN||14.5|g/dL|12.0-16.0|N|||F|||20240315115500\r\
OBX|4|NM|HCT^Hematocrit^LN||42.0|%|36.0-46.0|N|||F|||20240315115500\r\
OBX|5|NM|PLT^Platelet Count^LN||85|10*9/L|150-400|LL|||F|||20240315115500\r\
NTE|1|L|Patient appears slightly jaundiced. Recommend bilirubin panel.\r\
ORC|RE|ORD100002|LAB200002||CM||||20240315130000|||1234567^SMITH^JANE^M^MD\r\
OBR|2|ORD100002|LAB200002|CMP^Comprehensive Metabolic Panel^LN|||20240315120000|||||||||1234567^SMITH^JANE^M^MD||||||20240315130000|||F\r\
OBX|6|NM|GLU^Glucose^LN||185|mg/dL|70-110|HH|||F|||20240315125500\r\
OBX|7|NM|BUN^Blood Urea Nitrogen^LN||18|mg/dL|7-20|N|||F|||20240315125500\r\
OBX|8|NM|CREAT^Creatinine^LN||1.1|mg/dL|0.6-1.3|N|||F|||20240315125500\r\
OBX|9|ST|INTERP^Interpretation^LN||Elevated WBC suggests possible infection. Low platelets warrant follow-up.||||F|||20240315130000\r\
OBX|10|TX|COMMENT^Clinical Comment^LN||Patient currently on antibiotic therapy. CBC trending since admission. Platelet count declining - monitor for DIC. Consider hematology consult if PLT falls below 50.||||F|||20240315130000\r\
NTE|1|L|Critical value: Platelet count 85 - physician notified at 12:30.\r\
NTE|2|L|Glucose 185 - critical high - physician notified at 13:00.";

/// RDE^O11 pharmacy order with 2 medications, RXE/RXR/RXC/TQ1 (~1.1KB)
pub const RDE_O11_PHARMACY: &str = "\
MSH|^~\\&|PHARMACY|HOSPITAL|EMR|HOSPITAL|20240315160000||RDE^O11^RDE_O11|MSG00003|P|2.5|||AL|NE\r\
PID|1||MRN12345^^^HOSPITAL^MR||DOE^JOHN^ALLEN||19800315|M|||123 MAIN ST^^BOSTON^MA^02101^USA\r\
PV1|1|I|ICU^101^A^HOSPITAL||||1234567^SMITH^JANE^M^MD\r\
ORC|NW|RX001|RXFILL001||A||||20240315160000|||1234567^SMITH^JANE^M^MD\r\
RXE|1|12345^Amoxicillin 500mg^NDC|500|MG|CAP^Capsule^HL70292|||||30|CAP^Capsule||||||||||||TAKE ONE CAPSULE THREE TIMES DAILY WITH FOOD\r\
TQ1|1||TID^Three Times Daily^HL70335|||7^D^HL70255\r\
RXR|PO^Oral^HL70162\r\
RXC|B|67890^Clavulanic Acid 125mg^NDC|125|MG\r\
ORC|NW|RX002|RXFILL002||A||||20240315160000|||1234567^SMITH^JANE^M^MD\r\
RXE|2|54321^Metoprolol 25mg^NDC|25|MG|TAB^Tablet^HL70292|||||60|TAB^Tablet||||||||||||TAKE ONE TABLET TWICE DAILY\r\
TQ1|1||BID^Twice Daily^HL70335|||30^D^HL70255\r\
RXR|PO^Oral^HL70162";

/// SIU^S12 scheduling with SCH, AIG, AIL, AIP resource segments (~700B)
pub const SIU_S12_SCHEDULING: &str = "\
MSH|^~\\&|SCHED|HOSPITAL|EMR|HOSPITAL|20240315170000||SIU^S12^SIU_S12|MSG00004|P|2.5|||AL|NE\r\
SCH|APT001|APT001|||||ROUTINE^Routine^HL70276|FOLLOWUP^Follow Up^HL70277|30^MIN|MIN^Minutes^ISO+||1234567^SMITH^JANE^M^MD|^WPN^PH^^1^617^5551234||1234567^SMITH^JANE^M^MD|||||BOOKED^Booked^HL70278\r\
PID|1||MRN12345^^^HOSPITAL^MR||DOE^JOHN^ALLEN||19800315|M|||123 MAIN ST^^BOSTON^MA^02101^USA||^PRN^PH^^1^617^5551234\r\
PV1|1|O|CLINIC^201^A^HOSPITAL||||1234567^SMITH^JANE^M^MD\r\
AIG|1||1234567^SMITH^JANE^M^MD^HOSPITAL|P^Provider^HL70287|||20240320090000|20240320093000\r\
AIL|1||CLINIC^201^^HOSPITAL^^^^^EXAM ROOM 201|C^Clinic^HL70305|||20240320090000|20240320093000\r\
AIP|1||1234567^SMITH^JANE^M^MD|P^Provider^HL70287|||20240320090000|20240320093000";

/// MDM^T02 document notification with TXA + multiple OBX with long TX narrative (~4KB)
pub const MDM_T02_DOCUMENT: &str = "\
MSH|^~\\&|DICTATION|HOSPITAL|EMR|HOSPITAL|20240315180000||MDM^T02^MDM_T02|MSG00005|P|2.5|||AL|NE\r\
EVN|T02|20240315180000\r\
PID|1||MRN12345^^^HOSPITAL^MR||DOE^JOHN^ALLEN||19800315|M|||123 MAIN ST^^BOSTON^MA^02101^USA\r\
PV1|1|I|ICU^101^A^HOSPITAL||||1234567^SMITH^JANE^M^MD\r\
TXA|1|DS^Discharge Summary^HL70270|TX^Text^HL70191||20240315180000|||||1234567^SMITH^JANE^M^MD|DOC001|||||||AU^Authenticated^HL70271\r\
OBX|1|TX|DS^Discharge Summary^LN||DISCHARGE SUMMARY\\.br\\Patient: DOE, JOHN ALLEN\\.br\\MRN: 12345\\.br\\Admission Date: 03/15/2024\\.br\\Discharge Date: 03/15/2024\\.br\\\\.br\\CHIEF COMPLAINT:\\.br\\Chest pain radiating to left arm with associated diaphoresis and shortness of breath.\\.br\\\\.br\\HISTORY OF PRESENT ILLNESS:\\.br\\Mr. Doe is a 44-year-old male who presented to the Emergency Department with acute onset chest pain that began approximately 3 hours prior to arrival. The pain was described as substernal, pressure-like, 8/10 in severity, radiating to the left arm. Associated symptoms included diaphoresis, nausea, and dyspnea on exertion.||||F\r\
OBX|2|TX|DS^Discharge Summary^LN||HOSPITAL COURSE:\\.br\\The patient was admitted to the ICU for telemetry monitoring. Initial troponin was elevated at 0.45 ng/mL \\T\\ subsequent trending showed peak at 2.3 ng/mL. EKG demonstrated ST elevation in leads V1-V4. The patient underwent emergent cardiac catheterization which revealed 95% stenosis of the LAD. A drug-eluting stent was successfully placed with restoration of TIMI-3 flow.\\.br\\\\.br\\Post-procedure, the patient was started on dual antiplatelet therapy (aspirin 81mg daily \\T\\ clopidogrel 75mg daily). Echocardiogram showed EF of 45% with anterior wall hypokinesis. The patient remained hemodynamically stable throughout hospitalization.||||F\r\
OBX|3|TX|DS^Discharge Summary^LN||DISCHARGE MEDICATIONS:\\.br\\1. Aspirin 81mg PO daily\\.br\\2. Clopidogrel 75mg PO daily\\.br\\3. Metoprolol succinate 25mg PO daily\\.br\\4. Lisinopril 5mg PO daily\\.br\\5. Atorvastatin 80mg PO daily\\.br\\6. Nitroglycerin 0.4mg SL PRN chest pain\\.br\\\\.br\\FOLLOW-UP:\\.br\\Cardiology clinic in 1 week with Dr. Smith.\\.br\\Primary care in 2 weeks.\\.br\\Cardiac rehabilitation referral placed.||||F\r\
OBX|4|TX|DS^Discharge Summary^LN||DISCHARGE DIAGNOSES:\\.br\\1. Acute ST-elevation myocardial infarction (STEMI), LAD territory\\.br\\2. Coronary artery disease, single vessel\\.br\\3. Type 2 diabetes mellitus\\.br\\4. Essential hypertension\\.br\\5. Hyperlipidemia\\.br\\\\.br\\CONDITION AT DISCHARGE: Stable\\.br\\DISPOSITION: Home with home health services\\.br\\\\.br\\Electronically signed by:\\.br\\Jane M. Smith, MD\\.br\\Attending Physician, Internal Medicine\\.br\\Date: 03/15/2024||||F";

/// DFT^P03 financial transaction with multiple FT1, DG1, PR1 (~1.2KB)
pub const DFT_P03_FINANCIAL: &str = "\
MSH|^~\\&|BILLING|HOSPITAL|FINANCE|HOSPITAL|20240315190000||DFT^P03^DFT_P03|MSG00006|P|2.5|||AL|NE\r\
EVN|P03|20240315190000\r\
PID|1||MRN12345^^^HOSPITAL^MR||DOE^JOHN^ALLEN||19800315|M|||123 MAIN ST^^BOSTON^MA^02101^USA\r\
PV1|1|I|ICU^101^A^HOSPITAL||||1234567^SMITH^JANE^M^MD||||||||||||VN00001\r\
FT1|1|CHG001||20240315|20240315|CG|99223^Initial Hospital Care^CPT|||1|125.00||||||||||410.71^Acute Myocardial Infarction^ICD10\r\
FT1|2|CHG002||20240315|20240315|CG|93458^Cardiac Catheterization^CPT|||1|3500.00||||||||||I21.0^STEMI^ICD10\r\
FT1|3|CHG003||20240315|20240315|CG|92928^Percutaneous Coronary Stent^CPT|||1|15000.00||||||||||I21.0^STEMI^ICD10\r\
DG1|1|ICD10|I21.0^Acute transmural myocardial infarction of anterior wall^ICD10||20240315|AD\r\
DG1|2|ICD10|I25.10^Atherosclerotic heart disease^ICD10||20240315|AD\r\
DG1|3|ICD10|E11.9^Type 2 diabetes mellitus^ICD10||20240315|AD\r\
PR1|1|CPT|93458^Cardiac Catheterization^CPT|Cardiac Cath|20240315140000|||||1234567^SMITH^JANE^M^MD\r\
PR1|2|CPT|92928^Percutaneous Coronary Stent^CPT|PCI with Stent|20240315141500|||||1234567^SMITH^JANE^M^MD";

/// ORM^O01 order with 2 ORC/OBR order pairs, NTE notes (~800B)
pub const ORM_O01_ORDER: &str = "\
MSH|^~\\&|EMR|HOSPITAL|LAB|HOSPITAL|20240315200000||ORM^O01^ORM_O01|MSG00007|P|2.5|||AL|NE\r\
PID|1||MRN12345^^^HOSPITAL^MR||DOE^JOHN^ALLEN||19800315|M|||123 MAIN ST^^BOSTON^MA^02101^USA\r\
PV1|1|I|ICU^101^A^HOSPITAL||||1234567^SMITH^JANE^M^MD\r\
ORC|NW|ORD100001||ORD100001|IP||||20240315200000|||1234567^SMITH^JANE^M^MD\r\
OBR|1|ORD100001||CBC^Complete Blood Count^LN|||20240315200000|||||||||1234567^SMITH^JANE^M^MD\r\
NTE|1|P|STAT - Patient in ICU with declining platelets\r\
ORC|NW|ORD100002||ORD100002|IP||||20240315200000|||1234567^SMITH^JANE^M^MD\r\
OBR|2|ORD100002||CMP^Comprehensive Metabolic Panel^LN|||20240315200000|||||||||1234567^SMITH^JANE^M^MD\r\
NTE|1|P|Monitoring renal function post-catheterization contrast";

/// ADT^A01 with ~20% of values containing escape sequences (~400B)
pub const ESCAPE_HEAVY: &str = "\
MSH|^~\\&|HIS|HOSPITAL|EMR|HOSPITAL|20240315143000||ADT^A01^ADT_A01|MSG00008|P|2.5|||AL|NE\r\
PID|1||MRN12345^^^HOSPITAL^MR||O'BRIEN\\T\\SMITH^SEAN\\T\\JAMES||19800315|M|||123 MAIN ST \\T\\ APT 4B^^BOSTON^MA^02101^USA||^PRN^PH^^1^617^5551234\r\
PV1|1|I|ICU\\S\\CCU^101^A^HOSPITAL||||1234567^SMITH^JANE^^^MD\r\
AL1|1|DA|PCN\\E\\AMOX^Penicillin \\T\\ Amoxicillin^L|SV|Rash \\T\\ Anaphylaxis\r\
DG1|1|ICD10|I21.0^Acute MI (anterior wall)^ICD10\r\
NTE|1|L|Patient reports allergies to: PCN\\F\\AMOX\\F\\SULFA. BP reading: 120\\S\\80.";

/// ADT^A01 with trailing delimiters, empty components, extra whitespace (~500B)
pub const PRODUCTION_MESSY: &str = "\
MSH|^~\\&|HIS|HOSPITAL|||20240315143000||ADT^A01^ADT_A01|MSG00009|P|2.5||||||  \r\
PID|1||MRN12345^^^HOSPITAL^MR^||DOE^JOHN^^^||19800315|M|||123 MAIN ST^^BOSTON^MA^02101^^|||||||||||||||  \r\
PV1|1|I|ICU^101^^HOSPITAL^^^^||||1234567^SMITH^JANE^^^MD^^|||||||||||||||||||||||||||||||||||  \r\
OBX|1|NM|WBC^^LN||7.5|10*9/L|||||F|||  \r\
OBX|2|NM|||||||||||  ";

/// Generate a batch message (BHS/BTS envelope) with n ADT messages
pub fn generate_batch_message(n: usize) -> String {
    let mut msg = String::with_capacity(400 * n + 200);
    msg.push_str("BHS|^~\\&|HIS|HOSPITAL|EMR|HOSPITAL|20240315143000||BATCH001\r");

    for i in 1..=n {
        msg.push_str(&format!(
            "MSH|^~\\&|HIS|HOSPITAL|EMR|HOSPITAL|20240315143000||ADT^A01^ADT_A01|MSG{:05}|P|2.5\r\
PID|1||MRN{:05}^^^HOSPITAL^MR||PATIENT^TEST^{}||19800{}15|M|||{} MAIN ST^^BOSTON^MA^02101\r\
PV1|1|I|ICU^{}01^A||||1234567^SMITH^JANE^^^MD\r",
            i, i, i, (i % 12) + 1, i * 100, i
        ));
    }

    msg.push_str("BTS|");
    msg.push_str(&n.to_string());
    msg.push('\r');
    msg
}

/// Generate a file message (FHS/FTS envelope) with b batches of m messages each
pub fn generate_file_message(batches: usize, messages_per_batch: usize) -> String {
    let mut msg = String::with_capacity(400 * batches * messages_per_batch + 400);
    msg.push_str("FHS|^~\\&|HIS|HOSPITAL|EMR|HOSPITAL|20240315143000||FILE001\r");

    for b in 1..=batches {
        msg.push_str(&format!(
            "BHS|^~\\&|HIS|HOSPITAL|EMR|HOSPITAL|20240315143000||BATCH{:03}\r",
            b
        ));

        for m in 1..=messages_per_batch {
            let idx = (b - 1) * messages_per_batch + m;
            msg.push_str(&format!(
                "MSH|^~\\&|HIS|HOSPITAL|EMR|HOSPITAL|20240315143000||ADT^A01^ADT_A01|MSG{:05}|P|2.5\r\
PID|1||MRN{:05}^^^HOSPITAL^MR||PATIENT^TEST^{}||19800{}15|M\r\
PV1|1|I|ICU^{}01^A||||1234567^SMITH^JANE^^^MD\r",
                idx, idx, idx, (idx % 12) + 1, idx
            ));
        }

        msg.push_str(&format!("BTS|{}\r", messages_per_batch));
    }

    msg.push_str(&format!("FTS|{}\r", batches));
    msg
}
