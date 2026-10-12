#!/usr/bin/env python3
"""Generate complete HL7 v2.x schema files with all fields and component definitions.

This script generates JSON schema files for all supported HL7 message types
across versions v2.3, v2.4, v2.5, v2.6, and v2.7. Each schema includes:
- Complete segment field definitions per the HL7 standard
- Component definitions for composite data types
- Version-specific differences (fields added/removed, data type changes)
"""

import json
import os
import copy

# =============================================================================
# COMPOSITE DATA TYPE COMPONENT DEFINITIONS
# =============================================================================

# These define the internal structure of composite data types.
# Keys are 1-based component positions (as strings for JSON compatibility).

COMPONENTS_XPN = {
    "1": {"name": "Family Name", "data_type": "FN", "required": False, "max_length": 194},
    "2": {"name": "Given Name", "data_type": "ST", "required": False, "max_length": 30},
    "3": {"name": "Second and Further Given Names or Initials Thereof", "data_type": "ST", "required": False, "max_length": 30},
    "4": {"name": "Suffix", "data_type": "ST", "required": False, "max_length": 20},
    "5": {"name": "Prefix", "data_type": "ST", "required": False, "max_length": 20},
    "6": {"name": "Degree", "data_type": "IS", "required": False, "max_length": 6, "table_id": "0360"},
    "7": {"name": "Name Type Code", "data_type": "ID", "required": False, "max_length": 1, "table_id": "0200"},
    "8": {"name": "Name Representation Code", "data_type": "ID", "required": False, "max_length": 1, "table_id": "4000"},
    "9": {"name": "Name Context", "data_type": "CE", "required": False, "max_length": 483},
    "10": {"name": "Name Validity Range", "data_type": "DR", "required": False, "max_length": 53},
    "11": {"name": "Name Assembly Order", "data_type": "ID", "required": False, "max_length": 1, "table_id": "0444"},
    "12": {"name": "Effective Date", "data_type": "TS", "required": False, "max_length": 26},
    "13": {"name": "Expiration Date", "data_type": "TS", "required": False, "max_length": 26},
    "14": {"name": "Professional Suffix", "data_type": "ST", "required": False, "max_length": 199},
}

COMPONENTS_XAD = {
    "1": {"name": "Street Address", "data_type": "ST", "required": False, "max_length": 184},
    "2": {"name": "Other Designation", "data_type": "ST", "required": False, "max_length": 120},
    "3": {"name": "City", "data_type": "ST", "required": False, "max_length": 50},
    "4": {"name": "State or Province", "data_type": "ST", "required": False, "max_length": 50},
    "5": {"name": "Zip or Postal Code", "data_type": "ST", "required": False, "max_length": 12},
    "6": {"name": "Country", "data_type": "ID", "required": False, "max_length": 3, "table_id": "0399"},
    "7": {"name": "Address Type", "data_type": "ID", "required": False, "max_length": 3, "table_id": "0190"},
    "8": {"name": "Other Geographic Designation", "data_type": "ST", "required": False, "max_length": 50},
    "9": {"name": "County/Parish Code", "data_type": "IS", "required": False, "max_length": 20, "table_id": "0289"},
    "10": {"name": "Census Tract", "data_type": "IS", "required": False, "max_length": 20, "table_id": "0288"},
    "11": {"name": "Address Representation Code", "data_type": "ID", "required": False, "max_length": 1, "table_id": "4000"},
    "12": {"name": "Address Validity Range", "data_type": "DR", "required": False, "max_length": 53},
    "13": {"name": "Effective Date", "data_type": "TS", "required": False, "max_length": 26},
    "14": {"name": "Expiration Date", "data_type": "TS", "required": False, "max_length": 26},
}

COMPONENTS_XTN = {
    "1": {"name": "Telephone Number", "data_type": "ST", "required": False, "max_length": 199},
    "2": {"name": "Telecommunication Use Code", "data_type": "ID", "required": False, "max_length": 3, "table_id": "0201"},
    "3": {"name": "Telecommunication Equipment Type", "data_type": "ID", "required": False, "max_length": 8, "table_id": "0202"},
    "4": {"name": "Email Address", "data_type": "ST", "required": False, "max_length": 199},
    "5": {"name": "Country Code", "data_type": "NM", "required": False, "max_length": 3},
    "6": {"name": "Area/City Code", "data_type": "NM", "required": False, "max_length": 5},
    "7": {"name": "Local Number", "data_type": "NM", "required": False, "max_length": 9},
    "8": {"name": "Extension", "data_type": "NM", "required": False, "max_length": 5},
    "9": {"name": "Any Text", "data_type": "ST", "required": False, "max_length": 199},
    "10": {"name": "Extension Prefix", "data_type": "ST", "required": False, "max_length": 4},
    "11": {"name": "Speed Dial Code", "data_type": "ST", "required": False, "max_length": 6},
    "12": {"name": "Unformatted Telephone Number", "data_type": "ST", "required": False, "max_length": 199},
}

COMPONENTS_CX = {
    "1": {"name": "ID Number", "data_type": "ST", "required": True, "max_length": 15},
    "2": {"name": "Check Digit", "data_type": "ST", "required": False, "max_length": 1},
    "3": {"name": "Check Digit Scheme", "data_type": "ID", "required": False, "max_length": 3, "table_id": "0061"},
    "4": {"name": "Assigning Authority", "data_type": "HD", "required": False, "max_length": 227, "table_id": "0363"},
    "5": {"name": "Identifier Type Code", "data_type": "ID", "required": False, "max_length": 5, "table_id": "0203"},
    "6": {"name": "Assigning Facility", "data_type": "HD", "required": False, "max_length": 227},
    "7": {"name": "Effective Date", "data_type": "DT", "required": False, "max_length": 8},
    "8": {"name": "Expiration Date", "data_type": "DT", "required": False, "max_length": 8},
    "9": {"name": "Assigning Jurisdiction", "data_type": "CWE", "required": False, "max_length": 483},
    "10": {"name": "Assigning Agency or Department", "data_type": "CWE", "required": False, "max_length": 483},
}

COMPONENTS_XCN = {
    "1": {"name": "ID Number", "data_type": "ST", "required": False, "max_length": 15},
    "2": {"name": "Family Name", "data_type": "FN", "required": False, "max_length": 194},
    "3": {"name": "Given Name", "data_type": "ST", "required": False, "max_length": 30},
    "4": {"name": "Second and Further Given Names or Initials Thereof", "data_type": "ST", "required": False, "max_length": 30},
    "5": {"name": "Suffix", "data_type": "ST", "required": False, "max_length": 20},
    "6": {"name": "Prefix", "data_type": "ST", "required": False, "max_length": 20},
    "7": {"name": "Degree", "data_type": "IS", "required": False, "max_length": 6, "table_id": "0360"},
    "8": {"name": "Source Table", "data_type": "IS", "required": False, "max_length": 4, "table_id": "0297"},
    "9": {"name": "Assigning Authority", "data_type": "HD", "required": False, "max_length": 227, "table_id": "0363"},
    "10": {"name": "Name Type Code", "data_type": "ID", "required": False, "max_length": 1, "table_id": "0200"},
    "11": {"name": "Identifier Check Digit", "data_type": "ST", "required": False, "max_length": 1},
    "12": {"name": "Check Digit Scheme", "data_type": "ID", "required": False, "max_length": 3, "table_id": "0061"},
    "13": {"name": "Identifier Type Code", "data_type": "ID", "required": False, "max_length": 5, "table_id": "0203"},
    "14": {"name": "Assigning Facility", "data_type": "HD", "required": False, "max_length": 227},
    "15": {"name": "Name Representation Code", "data_type": "ID", "required": False, "max_length": 1, "table_id": "4000"},
    "16": {"name": "Name Context", "data_type": "CE", "required": False, "max_length": 483},
    "17": {"name": "Name Validity Range", "data_type": "DR", "required": False, "max_length": 53},
    "18": {"name": "Name Assembly Order", "data_type": "ID", "required": False, "max_length": 1, "table_id": "0444"},
    "19": {"name": "Effective Date", "data_type": "TS", "required": False, "max_length": 26},
    "20": {"name": "Expiration Date", "data_type": "TS", "required": False, "max_length": 26},
    "21": {"name": "Professional Suffix", "data_type": "ST", "required": False, "max_length": 199},
    "22": {"name": "Assigning Jurisdiction", "data_type": "CWE", "required": False, "max_length": 483},
    "23": {"name": "Assigning Agency or Department", "data_type": "CWE", "required": False, "max_length": 483},
}

COMPONENTS_HD = {
    "1": {"name": "Namespace ID", "data_type": "IS", "required": False, "max_length": 20, "table_id": "0300"},
    "2": {"name": "Universal ID", "data_type": "ST", "required": False, "max_length": 199},
    "3": {"name": "Universal ID Type", "data_type": "ID", "required": False, "max_length": 6, "table_id": "0301"},
}

COMPONENTS_CE = {
    "1": {"name": "Identifier", "data_type": "ST", "required": False, "max_length": 20},
    "2": {"name": "Text", "data_type": "ST", "required": False, "max_length": 199},
    "3": {"name": "Name of Coding System", "data_type": "ID", "required": False, "max_length": 20, "table_id": "0396"},
    "4": {"name": "Alternate Identifier", "data_type": "ST", "required": False, "max_length": 20},
    "5": {"name": "Alternate Text", "data_type": "ST", "required": False, "max_length": 199},
    "6": {"name": "Name of Alternate Coding System", "data_type": "ID", "required": False, "max_length": 20, "table_id": "0396"},
}

COMPONENTS_CWE = {
    "1": {"name": "Identifier", "data_type": "ST", "required": False, "max_length": 20},
    "2": {"name": "Text", "data_type": "ST", "required": False, "max_length": 199},
    "3": {"name": "Name of Coding System", "data_type": "ID", "required": False, "max_length": 20, "table_id": "0396"},
    "4": {"name": "Alternate Identifier", "data_type": "ST", "required": False, "max_length": 20},
    "5": {"name": "Alternate Text", "data_type": "ST", "required": False, "max_length": 199},
    "6": {"name": "Name of Alternate Coding System", "data_type": "ID", "required": False, "max_length": 20, "table_id": "0396"},
    "7": {"name": "Coding System Version ID", "data_type": "ST", "required": False, "max_length": 10},
    "8": {"name": "Alternate Coding System Version ID", "data_type": "ST", "required": False, "max_length": 10},
    "9": {"name": "Original Text", "data_type": "ST", "required": False, "max_length": 199},
}

COMPONENTS_MSG = {
    "1": {"name": "Message Code", "data_type": "ID", "required": True, "max_length": 3, "table_id": "0076"},
    "2": {"name": "Trigger Event", "data_type": "ID", "required": True, "max_length": 3, "table_id": "0003"},
    "3": {"name": "Message Structure", "data_type": "ID", "required": True, "max_length": 7, "table_id": "0354"},
}

COMPONENTS_PT = {
    "1": {"name": "Processing ID", "data_type": "ID", "required": True, "max_length": 1, "table_id": "0103"},
    "2": {"name": "Processing Mode", "data_type": "ID", "required": False, "max_length": 1, "table_id": "0207"},
}

COMPONENTS_VID = {
    "1": {"name": "Version ID", "data_type": "ID", "required": True, "max_length": 5, "table_id": "0104"},
    "2": {"name": "Internationalization Code", "data_type": "CE", "required": False, "max_length": 483},
    "3": {"name": "International Version ID", "data_type": "CE", "required": False, "max_length": 483},
}

COMPONENTS_PL = {
    "1": {"name": "Point of Care", "data_type": "IS", "required": False, "max_length": 20, "table_id": "0302"},
    "2": {"name": "Room", "data_type": "IS", "required": False, "max_length": 20, "table_id": "0303"},
    "3": {"name": "Bed", "data_type": "IS", "required": False, "max_length": 20, "table_id": "0304"},
    "4": {"name": "Facility", "data_type": "HD", "required": False, "max_length": 227},
    "5": {"name": "Location Status", "data_type": "IS", "required": False, "max_length": 20, "table_id": "0306"},
    "6": {"name": "Person Location Type", "data_type": "IS", "required": False, "max_length": 20, "table_id": "0305"},
    "7": {"name": "Building", "data_type": "IS", "required": False, "max_length": 20, "table_id": "0307"},
    "8": {"name": "Floor", "data_type": "IS", "required": False, "max_length": 20, "table_id": "0308"},
    "9": {"name": "Location Description", "data_type": "ST", "required": False, "max_length": 199},
    "10": {"name": "Comprehensive Location Identifier", "data_type": "EI", "required": False, "max_length": 427},
    "11": {"name": "Assigning Authority for Location", "data_type": "HD", "required": False, "max_length": 227},
}

COMPONENTS_EI = {
    "1": {"name": "Entity Identifier", "data_type": "ST", "required": False, "max_length": 199},
    "2": {"name": "Namespace ID", "data_type": "IS", "required": False, "max_length": 20, "table_id": "0363"},
    "3": {"name": "Universal ID", "data_type": "ST", "required": False, "max_length": 199},
    "4": {"name": "Universal ID Type", "data_type": "ID", "required": False, "max_length": 6, "table_id": "0301"},
}

COMPONENTS_CP = {
    "1": {"name": "Price", "data_type": "MO", "required": True, "max_length": 20},
    "2": {"name": "Price Type", "data_type": "ID", "required": False, "max_length": 2, "table_id": "0205"},
    "3": {"name": "From Value", "data_type": "NM", "required": False, "max_length": 16},
    "4": {"name": "To Value", "data_type": "NM", "required": False, "max_length": 16},
    "5": {"name": "Range Units", "data_type": "CE", "required": False, "max_length": 483},
    "6": {"name": "Range Type", "data_type": "ID", "required": False, "max_length": 1, "table_id": "0298"},
}

COMPONENTS_FC = {
    "1": {"name": "Financial Class Code", "data_type": "IS", "required": True, "max_length": 20, "table_id": "0064"},
    "2": {"name": "Effective Date", "data_type": "TS", "required": False, "max_length": 26},
}

COMPONENTS_DLD = {
    "1": {"name": "Discharge Location", "data_type": "IS", "required": True, "max_length": 20, "table_id": "0113"},
    "2": {"name": "Effective Date", "data_type": "TS", "required": False, "max_length": 26},
}

COMPONENTS_XON = {
    "1": {"name": "Organization Name", "data_type": "ST", "required": False, "max_length": 50},
    "2": {"name": "Organization Name Type Code", "data_type": "IS", "required": False, "max_length": 20, "table_id": "0204"},
    "3": {"name": "ID Number", "data_type": "NM", "required": False, "max_length": 4},
    "4": {"name": "Check Digit", "data_type": "NM", "required": False, "max_length": 1},
    "5": {"name": "Check Digit Scheme", "data_type": "ID", "required": False, "max_length": 3, "table_id": "0061"},
    "6": {"name": "Assigning Authority", "data_type": "HD", "required": False, "max_length": 227, "table_id": "0363"},
    "7": {"name": "Identifier Type Code", "data_type": "ID", "required": False, "max_length": 5, "table_id": "0203"},
    "8": {"name": "Assigning Facility", "data_type": "HD", "required": False, "max_length": 227},
    "9": {"name": "Name Representation Code", "data_type": "ID", "required": False, "max_length": 1, "table_id": "4000"},
    "10": {"name": "Organization Identifier", "data_type": "ST", "required": False, "max_length": 20},
}

COMPONENTS_CQ = {
    "1": {"name": "Quantity", "data_type": "NM", "required": False, "max_length": 16},
    "2": {"name": "Units", "data_type": "CE", "required": False, "max_length": 483},
}

COMPONENTS_MO = {
    "1": {"name": "Quantity", "data_type": "NM", "required": False, "max_length": 16},
    "2": {"name": "Denomination", "data_type": "ID", "required": False, "max_length": 3, "table_id": "ISO4217"},
}

COMPONENTS_DR = {
    "1": {"name": "Range Start Date/Time", "data_type": "TS", "required": False, "max_length": 26},
    "2": {"name": "Range End Date/Time", "data_type": "TS", "required": False, "max_length": 26},
}

COMPONENTS_SN = {
    "1": {"name": "Comparator", "data_type": "ST", "required": False, "max_length": 2},
    "2": {"name": "Num1", "data_type": "NM", "required": False, "max_length": 15},
    "3": {"name": "Separator/Suffix", "data_type": "ST", "required": False, "max_length": 1},
    "4": {"name": "Num2", "data_type": "NM", "required": False, "max_length": 15},
}

COMPONENTS_TQ = {
    "1": {"name": "Quantity", "data_type": "CQ", "required": False, "max_length": 267},
    "2": {"name": "Interval", "data_type": "ST", "required": False, "max_length": 206},
    "3": {"name": "Duration", "data_type": "ST", "required": False, "max_length": 6},
    "4": {"name": "Start Date/Time", "data_type": "TS", "required": False, "max_length": 26},
    "5": {"name": "End Date/Time", "data_type": "TS", "required": False, "max_length": 26},
    "6": {"name": "Priority", "data_type": "ST", "required": False, "max_length": 6},
    "7": {"name": "Condition", "data_type": "ST", "required": False, "max_length": 199},
    "8": {"name": "Text", "data_type": "TX", "required": False, "max_length": 200},
    "9": {"name": "Conjunction", "data_type": "ID", "required": False, "max_length": 1, "table_id": "0472"},
    "10": {"name": "Order Sequencing", "data_type": "ST", "required": False, "max_length": 110},
    "11": {"name": "Occurrence Duration", "data_type": "CE", "required": False, "max_length": 483},
    "12": {"name": "Total Occurrences", "data_type": "NM", "required": False, "max_length": 4},
}

COMPONENTS_JCC = {
    "1": {"name": "Job Code", "data_type": "IS", "required": False, "max_length": 20, "table_id": "0327"},
    "2": {"name": "Job Class", "data_type": "IS", "required": False, "max_length": 20, "table_id": "0328"},
    "3": {"name": "Job Description Text", "data_type": "TX", "required": False, "max_length": 250},
}

COMPONENTS_FN = {
    "1": {"name": "Surname", "data_type": "ST", "required": True, "max_length": 50},
    "2": {"name": "Own Surname Prefix", "data_type": "ST", "required": False, "max_length": 20},
    "3": {"name": "Own Surname", "data_type": "ST", "required": False, "max_length": 50},
    "4": {"name": "Surname Prefix From Partner/Spouse", "data_type": "ST", "required": False, "max_length": 20},
    "5": {"name": "Surname From Partner/Spouse", "data_type": "ST", "required": False, "max_length": 50},
}

COMPONENTS_EIP = {
    "1": {"name": "Placer Assigned Identifier", "data_type": "EI", "required": False, "max_length": 427},
    "2": {"name": "Filler Assigned Identifier", "data_type": "EI", "required": False, "max_length": 427},
}

# Map data type codes to their component definitions
COMPOSITE_COMPONENTS = {
    "XPN": COMPONENTS_XPN,
    "XAD": COMPONENTS_XAD,
    "XTN": COMPONENTS_XTN,
    "CX": COMPONENTS_CX,
    "XCN": COMPONENTS_XCN,
    "HD": COMPONENTS_HD,
    "CE": COMPONENTS_CE,
    "CWE": COMPONENTS_CWE,
    "MSG": COMPONENTS_MSG,
    "PT": COMPONENTS_PT,
    "VID": COMPONENTS_VID,
    "PL": COMPONENTS_PL,
    "EI": COMPONENTS_EI,
    "CP": COMPONENTS_CP,
    "FC": COMPONENTS_FC,
    "DLD": COMPONENTS_DLD,
    "XON": COMPONENTS_XON,
    "CQ": COMPONENTS_CQ,
    "MO": COMPONENTS_MO,
    "DR": COMPONENTS_DR,
    "SN": COMPONENTS_SN,
    "TQ": COMPONENTS_TQ,
    "JCC": COMPONENTS_JCC,
    "FN": COMPONENTS_FN,
    "EIP": COMPONENTS_EIP,
}


def field(name, data_type, required=False, repeating=False, max_length=None, table_id=None):
    """Create a field definition dict."""
    f = {
        "name": name,
        "data_type": data_type,
        "required": required,
        "repeating": repeating,
    }
    if max_length is not None:
        f["max_length"] = max_length
    if table_id is not None:
        f["table_id"] = table_id
    # Add component definitions for composite types
    if data_type in COMPOSITE_COMPONENTS:
        f["components"] = copy.deepcopy(COMPOSITE_COMPONENTS[data_type])
    return f


# =============================================================================
# COMPLETE SEGMENT DEFINITIONS (HL7 v2.5 as baseline)
# =============================================================================

def get_msh_fields():
    return {
        "1": field("Field Separator", "ST", required=True, max_length=1),
        "2": field("Encoding Characters", "ST", required=True, max_length=4),
        "3": field("Sending Application", "HD", max_length=227),
        "4": field("Sending Facility", "HD", max_length=227),
        "5": field("Receiving Application", "HD", max_length=227),
        "6": field("Receiving Facility", "HD", max_length=227),
        "7": field("Date/Time Of Message", "TS", max_length=26),
        "8": field("Security", "ST", max_length=40),
        "9": field("Message Type", "MSG", required=True, max_length=15),
        "10": field("Message Control ID", "ST", required=True, max_length=20),
        "11": field("Processing ID", "PT", required=True, max_length=3),
        "12": field("Version ID", "VID", required=True, max_length=60),
        "13": field("Sequence Number", "NM", max_length=15),
        "14": field("Continuation Pointer", "ST", max_length=180),
        "15": field("Accept Acknowledgment Type", "ID", max_length=2, table_id="0155"),
        "16": field("Application Acknowledgment Type", "ID", max_length=2, table_id="0155"),
        "17": field("Country Code", "ID", max_length=3, table_id="0399"),
        "18": field("Character Set", "ID", repeating=True, max_length=16, table_id="0211"),
        "19": field("Principal Language Of Message", "CE", max_length=250),
        "20": field("Alternate Character Set Handling Scheme", "ID", max_length=20, table_id="0356"),
        "21": field("Message Profile Identifier", "EI", repeating=True, max_length=427),
    }


def get_evn_fields():
    return {
        "1": field("Event Type Code", "ID", max_length=3, table_id="0003"),
        "2": field("Recorded Date/Time", "TS", max_length=26),
        "3": field("Date/Time Planned Event", "TS", max_length=26),
        "4": field("Event Reason Code", "IS", max_length=3, table_id="0062"),
        "5": field("Operator ID", "XCN", repeating=True, max_length=250),
        "6": field("Event Occurred", "TS", max_length=26),
        "7": field("Event Facility", "HD", max_length=241),
    }


def get_pid_fields():
    return {
        "1": field("Set ID - PID", "SI", max_length=4),
        "2": field("Patient ID", "CX", max_length=20),
        "3": field("Patient Identifier List", "CX", required=True, repeating=True, max_length=250),
        "4": field("Alternate Patient ID - PID", "CX", repeating=True, max_length=20),
        "5": field("Patient Name", "XPN", required=True, repeating=True, max_length=250),
        "6": field("Mother's Maiden Name", "XPN", repeating=True, max_length=250),
        "7": field("Date/Time of Birth", "TS", max_length=26),
        "8": field("Administrative Sex", "IS", max_length=1, table_id="0001"),
        "9": field("Patient Alias", "XPN", repeating=True, max_length=250),
        "10": field("Race", "CE", repeating=True, max_length=250, table_id="0005"),
        "11": field("Patient Address", "XAD", repeating=True, max_length=250),
        "12": field("County Code", "IS", max_length=4, table_id="0289"),
        "13": field("Phone Number - Home", "XTN", repeating=True, max_length=250),
        "14": field("Phone Number - Business", "XTN", repeating=True, max_length=250),
        "15": field("Primary Language", "CE", max_length=250, table_id="0296"),
        "16": field("Marital Status", "CE", max_length=250, table_id="0002"),
        "17": field("Religion", "CE", max_length=250, table_id="0006"),
        "18": field("Patient Account Number", "CX", max_length=250),
        "19": field("SSN Number - Patient", "ST", max_length=16),
        "20": field("Driver's License Number - Patient", "ST", max_length=25),
        "21": field("Mother's Identifier", "CX", repeating=True, max_length=250),
        "22": field("Ethnic Group", "CE", repeating=True, max_length=250, table_id="0189"),
        "23": field("Birth Place", "ST", max_length=250),
        "24": field("Multiple Birth Indicator", "ID", max_length=1, table_id="0136"),
        "25": field("Birth Order", "NM", max_length=2),
        "26": field("Citizenship", "CE", repeating=True, max_length=250, table_id="0171"),
        "27": field("Veterans Military Status", "CE", max_length=250, table_id="0172"),
        "28": field("Nationality", "CE", max_length=250, table_id="0212"),
        "29": field("Patient Death Date and Time", "TS", max_length=26),
        "30": field("Patient Death Indicator", "ID", max_length=1, table_id="0136"),
        "31": field("Identity Unknown Indicator", "ID", max_length=1, table_id="0136"),
        "32": field("Identity Reliability Code", "IS", repeating=True, max_length=20, table_id="0445"),
        "33": field("Last Update Date/Time", "TS", max_length=26),
        "34": field("Last Update Facility", "HD", max_length=241),
        "35": field("Species Code", "CE", max_length=250, table_id="0446"),
        "36": field("Breed Code", "CE", max_length=250, table_id="0447"),
        "37": field("Strain", "ST", max_length=80),
        "38": field("Production Class Code", "CE", repeating=True, max_length=250, table_id="0429"),
        "39": field("Tribal Citizenship", "CWE", repeating=True, max_length=250, table_id="0171"),
    }


def get_pv1_fields():
    return {
        "1": field("Set ID - PV1", "SI", max_length=4),
        "2": field("Patient Class", "IS", required=True, max_length=1, table_id="0004"),
        "3": field("Assigned Patient Location", "PL", max_length=80),
        "4": field("Admission Type", "IS", max_length=2, table_id="0007"),
        "5": field("Preadmit Number", "CX", max_length=250),
        "6": field("Prior Patient Location", "PL", max_length=80),
        "7": field("Attending Doctor", "XCN", repeating=True, max_length=250, table_id="0010"),
        "8": field("Referring Doctor", "XCN", repeating=True, max_length=250, table_id="0010"),
        "9": field("Consulting Doctor", "XCN", repeating=True, max_length=250, table_id="0010"),
        "10": field("Hospital Service", "IS", max_length=3, table_id="0069"),
        "11": field("Temporary Location", "PL", max_length=80),
        "12": field("Preadmit Test Indicator", "IS", max_length=2, table_id="0087"),
        "13": field("Re-admission Indicator", "IS", max_length=2, table_id="0092"),
        "14": field("Admit Source", "IS", max_length=6, table_id="0023"),
        "15": field("Ambulatory Status", "IS", repeating=True, max_length=2, table_id="0009"),
        "16": field("VIP Indicator", "IS", max_length=2, table_id="0099"),
        "17": field("Admitting Doctor", "XCN", repeating=True, max_length=250, table_id="0010"),
        "18": field("Patient Type", "IS", max_length=2, table_id="0018"),
        "19": field("Visit Number", "CX", max_length=250),
        "20": field("Financial Class", "FC", repeating=True, max_length=50, table_id="0064"),
        "21": field("Charge Price Indicator", "IS", max_length=2, table_id="0032"),
        "22": field("Courtesy Code", "IS", max_length=2, table_id="0045"),
        "23": field("Credit Rating", "IS", max_length=2, table_id="0046"),
        "24": field("Contract Code", "IS", repeating=True, max_length=2, table_id="0044"),
        "25": field("Contract Effective Date", "DT", repeating=True, max_length=8),
        "26": field("Contract Amount", "NM", repeating=True, max_length=12),
        "27": field("Contract Period", "NM", repeating=True, max_length=3),
        "28": field("Interest Code", "IS", max_length=2, table_id="0073"),
        "29": field("Transfer to Bad Debt Code", "IS", max_length=4, table_id="0110"),
        "30": field("Transfer to Bad Debt Date", "DT", max_length=8),
        "31": field("Bad Debt Agency Code", "IS", max_length=10, table_id="0021"),
        "32": field("Bad Debt Transfer Amount", "NM", max_length=12),
        "33": field("Bad Debt Recovery Amount", "NM", max_length=12),
        "34": field("Delete Account Indicator", "IS", max_length=1, table_id="0111"),
        "35": field("Delete Account Date", "DT", max_length=8),
        "36": field("Discharge Disposition", "IS", max_length=3, table_id="0112"),
        "37": field("Discharged to Location", "DLD", max_length=47, table_id="0113"),
        "38": field("Diet Type", "CE", max_length=250, table_id="0114"),
        "39": field("Servicing Facility", "IS", max_length=2, table_id="0115"),
        "40": field("Bed Status", "IS", max_length=1, table_id="0116"),
        "41": field("Account Status", "IS", max_length=2, table_id="0117"),
        "42": field("Pending Location", "PL", max_length=80),
        "43": field("Prior Temporary Location", "PL", max_length=80),
        "44": field("Admit Date/Time", "TS", max_length=26),
        "45": field("Discharge Date/Time", "TS", repeating=True, max_length=26),
        "46": field("Current Patient Balance", "NM", max_length=12),
        "47": field("Total Charges", "NM", max_length=12),
        "48": field("Total Adjustments", "NM", max_length=12),
        "49": field("Total Payments", "NM", max_length=12),
        "50": field("Alternate Visit ID", "CX", max_length=250),
        "51": field("Visit Indicator", "IS", max_length=1, table_id="0326"),
        "52": field("Other Healthcare Provider", "XCN", repeating=True, max_length=250, table_id="0010"),
    }


def get_obr_fields():
    return {
        "1": field("Set ID - OBR", "SI", max_length=4),
        "2": field("Placer Order Number", "EI", max_length=22),
        "3": field("Filler Order Number", "EI", max_length=22),
        "4": field("Universal Service Identifier", "CE", required=True, max_length=250),
        "5": field("Priority - OBR", "ID", max_length=2),
        "6": field("Requested Date/Time", "TS", max_length=26),
        "7": field("Observation Date/Time", "TS", max_length=26),
        "8": field("Observation End Date/Time", "TS", max_length=26),
        "9": field("Collection Volume", "CQ", max_length=20),
        "10": field("Collector Identifier", "XCN", repeating=True, max_length=250),
        "11": field("Specimen Action Code", "ID", max_length=1, table_id="0065"),
        "12": field("Danger Code", "CE", max_length=250),
        "13": field("Relevant Clinical Information", "ST", max_length=300),
        "14": field("Specimen Received Date/Time", "TS", max_length=26),
        "15": field("Specimen Source", "ST", max_length=300),
        "16": field("Ordering Provider", "XCN", repeating=True, max_length=250),
        "17": field("Order Callback Phone Number", "XTN", repeating=True, max_length=250),
        "18": field("Placer Field 1", "ST", max_length=60),
        "19": field("Placer Field 2", "ST", max_length=60),
        "20": field("Filler Field 1", "ST", max_length=60),
        "21": field("Filler Field 2", "ST", max_length=60),
        "22": field("Results Rpt/Status Chng - Date/Time", "TS", max_length=26),
        "23": field("Charge to Practice", "MO", max_length=40),
        "24": field("Diagnostic Serv Sect ID", "ID", max_length=10, table_id="0074"),
        "25": field("Result Status", "ID", max_length=1, table_id="0123"),
        "26": field("Parent Result", "ST", max_length=400),
        "27": field("Quantity/Timing", "TQ", repeating=True, max_length=200),
        "28": field("Result Copies To", "XCN", repeating=True, max_length=250),
        "29": field("Parent", "EIP", max_length=200),
        "30": field("Transportation Mode", "ID", max_length=20, table_id="0124"),
        "31": field("Reason for Study", "CE", repeating=True, max_length=250),
        "32": field("Principal Result Interpreter", "ST", max_length=200),
        "33": field("Assistant Result Interpreter", "ST", repeating=True, max_length=200),
        "34": field("Technician", "ST", repeating=True, max_length=200),
        "35": field("Transcriptionist", "ST", repeating=True, max_length=200),
        "36": field("Scheduled Date/Time", "TS", max_length=26),
        "37": field("Number of Sample Containers", "NM", max_length=4),
        "38": field("Transport Logistics of Collected Sample", "CE", repeating=True, max_length=250),
        "39": field("Collector's Comment", "CE", repeating=True, max_length=250),
        "40": field("Transport Arrangement Responsibility", "CE", max_length=250),
        "41": field("Transport Arranged", "ID", max_length=30, table_id="0224"),
        "42": field("Escort Required", "ID", max_length=1, table_id="0225"),
        "43": field("Planned Patient Transport Comment", "CE", repeating=True, max_length=250),
        "44": field("Procedure Code", "CE", max_length=250, table_id="0088"),
        "45": field("Procedure Code Modifier", "CE", repeating=True, max_length=250, table_id="0340"),
        "46": field("Placer Supplemental Service Information", "CE", repeating=True, max_length=250, table_id="0411"),
        "47": field("Filler Supplemental Service Information", "CE", repeating=True, max_length=250, table_id="0411"),
        "48": field("Medically Necessary Duplicate Procedure Reason", "CWE", max_length=250),
        "49": field("Result Handling", "IS", max_length=2, table_id="0507"),
    }


def get_obx_fields():
    return {
        "1": field("Set ID - OBX", "SI", max_length=4),
        "2": field("Value Type", "ID", max_length=2, table_id="0125"),
        "3": field("Observation Identifier", "CE", required=True, max_length=250),
        "4": field("Observation Sub-ID", "ST", max_length=20),
        "5": field("Observation Value", "Varies", repeating=True, max_length=65536),
        "6": field("Units", "CE", max_length=250),
        "7": field("References Range", "ST", max_length=60),
        "8": field("Abnormal Flags", "IS", repeating=True, max_length=5, table_id="0078"),
        "9": field("Probability", "NM", max_length=5),
        "10": field("Nature of Abnormal Test", "ID", repeating=True, max_length=2, table_id="0080"),
        "11": field("Observation Result Status", "ID", required=True, max_length=1, table_id="0085"),
        "12": field("Effective Date of Reference Range", "TS", max_length=26),
        "13": field("User Defined Access Checks", "ST", max_length=20),
        "14": field("Date/Time of the Observation", "TS", max_length=26),
        "15": field("Producer's ID", "CE", max_length=250),
        "16": field("Responsible Observer", "XCN", repeating=True, max_length=250),
        "17": field("Observation Method", "CE", repeating=True, max_length=250),
        "18": field("Equipment Instance Identifier", "EI", repeating=True, max_length=22),
        "19": field("Date/Time of the Analysis", "TS", max_length=26),
    }


def get_orc_fields():
    return {
        "1": field("Order Control", "ID", required=True, max_length=2, table_id="0119"),
        "2": field("Placer Order Number", "EI", max_length=22),
        "3": field("Filler Order Number", "EI", max_length=22),
        "4": field("Placer Group Number", "EI", max_length=22),
        "5": field("Order Status", "ID", max_length=2, table_id="0038"),
        "6": field("Response Flag", "ID", max_length=1, table_id="0121"),
        "7": field("Quantity/Timing", "TQ", repeating=True, max_length=200),
        "8": field("Parent", "EIP", max_length=200),
        "9": field("Date/Time of Transaction", "TS", max_length=26),
        "10": field("Entered By", "XCN", repeating=True, max_length=250),
        "11": field("Verified By", "XCN", repeating=True, max_length=250),
        "12": field("Ordering Provider", "XCN", repeating=True, max_length=250),
        "13": field("Enterer's Location", "PL", max_length=80),
        "14": field("Call Back Phone Number", "XTN", repeating=True, max_length=250),
        "15": field("Order Effective Date/Time", "TS", max_length=26),
        "16": field("Order Control Code Reason", "CE", max_length=250),
        "17": field("Entering Organization", "CE", max_length=250),
        "18": field("Entering Device", "CE", max_length=250),
        "19": field("Action By", "XCN", repeating=True, max_length=250),
        "20": field("Advanced Beneficiary Notice Code", "CE", max_length=250, table_id="0339"),
        "21": field("Ordering Facility Name", "XON", repeating=True, max_length=250),
        "22": field("Ordering Facility Address", "XAD", repeating=True, max_length=250),
        "23": field("Ordering Facility Phone Number", "XTN", repeating=True, max_length=250),
        "24": field("Ordering Provider Address", "XAD", repeating=True, max_length=250),
        "25": field("Order Status Modifier", "CWE", max_length=250),
        "26": field("Advanced Beneficiary Notice Override Reason", "CWE", max_length=60, table_id="0552"),
        "27": field("Filler's Expected Availability Date/Time", "TS", max_length=26),
        "28": field("Confidentiality Code", "CWE", max_length=250, table_id="0177"),
        "29": field("Order Type", "CWE", max_length=250, table_id="0482"),
        "30": field("Enterer Authorization Mode", "CNE", max_length=250, table_id="0483"),
        "31": field("Parent Universal Service Identifier", "CWE", max_length=250),
    }


def get_msa_fields():
    return {
        "1": field("Acknowledgment Code", "ID", required=True, max_length=2, table_id="0008"),
        "2": field("Message Control ID", "ST", required=True, max_length=20),
        "3": field("Text Message", "ST", max_length=80),
        "4": field("Expected Sequence Number", "NM", max_length=15),
        "5": field("Delayed Acknowledgment Type", "ID", max_length=1),
        "6": field("Error Condition", "CE", max_length=250),
    }


def get_err_fields():
    return {
        "1": field("Error Code and Location", "ST", max_length=80),
        "2": field("Error Location", "ST", repeating=True, max_length=18),
        "3": field("HL7 Error Code", "CWE", required=True, max_length=705, table_id="0357"),
        "4": field("Severity", "ID", required=True, max_length=2, table_id="0516"),
    }


def get_mrg_fields():
    return {
        "1": field("Prior Patient Identifier List", "CX", required=True, repeating=True, max_length=250),
        "2": field("Prior Alternate Patient ID", "CX", repeating=True, max_length=250),
        "3": field("Prior Patient Account Number", "CX", max_length=250),
        "4": field("Prior Patient ID", "CX", max_length=250),
        "5": field("Prior Visit Number", "CX", max_length=250),
        "6": field("Prior Alternate Visit ID", "CX", max_length=250),
        "7": field("Prior Patient Name", "XPN", repeating=True, max_length=250),
    }


def get_ft1_fields():
    return {
        "1": field("Set ID - FT1", "SI", max_length=4),
        "2": field("Transaction ID", "ST", max_length=12),
        "3": field("Transaction Batch ID", "ST", max_length=10),
        "4": field("Transaction Date", "DR", required=True, max_length=53),
        "5": field("Transaction Posting Date", "TS", max_length=26),
        "6": field("Transaction Type", "IS", required=True, max_length=8, table_id="0017"),
        "7": field("Transaction Code", "CE", required=True, max_length=250, table_id="0132"),
        "8": field("Transaction Description", "ST", max_length=40),
        "9": field("Transaction Description - Alt", "ST", max_length=40),
        "10": field("Transaction Quantity", "NM", max_length=6),
        "11": field("Transaction Amount - Extended", "CP", max_length=12),
        "12": field("Transaction Amount - Unit", "CP", max_length=12),
        "13": field("Department Code", "CE", max_length=250, table_id="0049"),
        "14": field("Insurance Plan ID", "CE", max_length=250, table_id="0072"),
        "15": field("Insurance Amount", "CP", max_length=12),
        "16": field("Assigned Patient Location", "PL", max_length=80),
        "17": field("Fee Schedule", "IS", max_length=1, table_id="0024"),
        "18": field("Patient Type", "IS", max_length=2, table_id="0018"),
        "19": field("Diagnosis Code - FT1", "CE", repeating=True, max_length=250, table_id="0051"),
        "20": field("Performed By Code", "XCN", repeating=True, max_length=250),
        "21": field("Ordered By Code", "XCN", repeating=True, max_length=250),
        "22": field("Unit Cost", "CP", max_length=12),
        "23": field("Filler Order Number", "EI", max_length=22),
        "24": field("Entered By Code", "XCN", repeating=True, max_length=250),
        "25": field("Procedure Code", "CE", max_length=250, table_id="0088"),
        "26": field("Procedure Code Modifier", "CE", repeating=True, max_length=250, table_id="0340"),
        "27": field("Advanced Beneficiary Notice Code", "CE", max_length=250, table_id="0339"),
        "28": field("Medically Necessary Duplicate Procedure Reason", "CWE", max_length=250),
        "29": field("NDC Code", "CNE", max_length=250, table_id="0549"),
        "30": field("Payment Reference ID", "CX", max_length=250),
        "31": field("Transaction Reference Key", "SI", repeating=True, max_length=4),
    }


def get_txa_fields():
    return {
        "1": field("Set ID- TXA", "SI", required=True, max_length=4),
        "2": field("Document Type", "IS", required=True, max_length=30, table_id="0270"),
        "3": field("Document Content Presentation", "ID", max_length=2, table_id="0191"),
        "4": field("Activity Date/Time", "TS", max_length=26),
        "5": field("Primary Activity Provider Code/Name", "XCN", repeating=True, max_length=250),
        "6": field("Origination Date/Time", "TS", max_length=26),
        "7": field("Transcription Date/Time", "TS", max_length=26),
        "8": field("Edit Date/Time", "TS", repeating=True, max_length=26),
        "9": field("Originator Code/Name", "XCN", repeating=True, max_length=250),
        "10": field("Assigned Document Authenticator", "XCN", repeating=True, max_length=250),
        "11": field("Transcriptionist Code/Name", "XCN", repeating=True, max_length=250),
        "12": field("Unique Document Number", "EI", required=True, max_length=30),
        "13": field("Parent Document Number", "EI", max_length=30),
        "14": field("Placer Order Number", "EI", repeating=True, max_length=22),
        "15": field("Filler Order Number", "EI", max_length=22),
        "16": field("Unique Document File Name", "ST", max_length=30),
        "17": field("Document Completion Status", "ID", required=True, max_length=2, table_id="0271"),
        "18": field("Document Confidentiality Status", "ID", max_length=2, table_id="0272"),
        "19": field("Document Availability Status", "ID", max_length=2, table_id="0273"),
        "20": field("Document Storage Status", "ID", max_length=2, table_id="0275"),
        "21": field("Document Change Reason", "ST", max_length=30),
        "22": field("Authentication Person, Time Stamp", "ST", repeating=True, max_length=250),
        "23": field("Distributed Copies (Code and Name of Recipients)", "XCN", repeating=True, max_length=250),
    }


def get_sch_fields():
    return {
        "1": field("Placer Appointment ID", "EI", max_length=75),
        "2": field("Filler Appointment ID", "EI", max_length=75),
        "3": field("Occurrence Number", "NM", max_length=5),
        "4": field("Placer Group Number", "EI", max_length=22),
        "5": field("Schedule ID", "CE", max_length=250),
        "6": field("Event Reason", "CE", required=True, max_length=250),
        "7": field("Appointment Reason", "CE", max_length=250, table_id="0276"),
        "8": field("Appointment Type", "CE", max_length=250, table_id="0277"),
        "9": field("Appointment Duration", "NM", max_length=20),
        "10": field("Appointment Duration Units", "CE", max_length=250),
        "11": field("Appointment Timing Quantity", "TQ", repeating=True, max_length=200),
        "12": field("Placer Contact Person", "XCN", repeating=True, max_length=250),
        "13": field("Placer Contact Phone Number", "XTN", max_length=250),
        "14": field("Placer Contact Address", "XAD", repeating=True, max_length=250),
        "15": field("Placer Contact Location", "PL", max_length=80),
        "16": field("Filler Contact Person", "XCN", repeating=True, max_length=250),
        "17": field("Filler Contact Phone Number", "XTN", max_length=250),
        "18": field("Filler Contact Address", "XAD", repeating=True, max_length=250),
        "19": field("Filler Contact Location", "PL", max_length=80),
        "20": field("Entered By Person", "XCN", repeating=True, max_length=250),
        "21": field("Entered By Phone Number", "XTN", repeating=True, max_length=250),
        "22": field("Entered By Location", "PL", max_length=80),
        "23": field("Parent Placer Appointment ID", "EI", max_length=75),
        "24": field("Parent Filler Appointment ID", "EI", max_length=75),
        "25": field("Filler Status Code", "CE", max_length=250, table_id="0278"),
        "26": field("Placer Order Number", "EI", repeating=True, max_length=22),
        "27": field("Filler Order Number", "EI", repeating=True, max_length=22),
    }


def get_qrd_fields():
    return {
        "1": field("Query Date/Time", "TS", required=True, max_length=26),
        "2": field("Query Format Code", "ID", required=True, max_length=1, table_id="0106"),
        "3": field("Query Priority", "ID", required=True, max_length=1, table_id="0091"),
        "4": field("Query ID", "ST", required=True, max_length=10),
        "5": field("Deferred Response Type", "ID", max_length=1, table_id="0107"),
        "6": field("Deferred Response Date/Time", "TS", max_length=26),
        "7": field("Quantity Limited Request", "CQ", required=True, max_length=10),
        "8": field("Who Subject Filter", "XCN", required=True, repeating=True, max_length=250),
        "9": field("What Subject Filter", "CE", required=True, repeating=True, max_length=250, table_id="0048"),
        "10": field("What Department Data Code", "CE", repeating=True, max_length=250),
        "11": field("What Data Code Value Qual.", "ST", repeating=True, max_length=20),
        "12": field("Query Results Level", "ID", max_length=1, table_id="0108"),
    }


def get_qrf_fields():
    return {
        "1": field("Where Subject Filter", "ST", required=True, repeating=True, max_length=20),
        "2": field("When Data Start Date/Time", "TS", max_length=26),
        "3": field("When Data End Date/Time", "TS", max_length=26),
        "4": field("What User Qualifier", "ST", repeating=True, max_length=60),
        "5": field("Other QRY Subject Filter", "ST", repeating=True, max_length=60),
        "6": field("Which Date/Time Qualifier", "ID", repeating=True, max_length=12, table_id="0156"),
        "7": field("Which Date/Time Status Qualifier", "ID", repeating=True, max_length=12, table_id="0157"),
        "8": field("Date/Time Selection Qualifier", "ID", repeating=True, max_length=12, table_id="0158"),
        "9": field("When Quantity/Timing Qualifier", "TQ", max_length=60),
        "10": field("Search Confidence Threshold", "NM", max_length=10),
    }


def get_rxa_fields():
    return {
        "1": field("Give Sub-ID Counter", "NM", required=True, max_length=4),
        "2": field("Administration Sub-ID Counter", "NM", required=True, max_length=4),
        "3": field("Date/Time Start of Administration", "TS", required=True, max_length=26),
        "4": field("Date/Time End of Administration", "TS", required=True, max_length=26),
        "5": field("Administered Code", "CE", required=True, max_length=250, table_id="0292"),
        "6": field("Administered Amount", "NM", required=True, max_length=20),
        "7": field("Administered Units", "CE", max_length=250),
        "8": field("Administered Dosage Form", "CE", max_length=250),
        "9": field("Administration Notes", "CE", repeating=True, max_length=250),
        "10": field("Administering Provider", "XCN", repeating=True, max_length=250),
        "11": field("Administered-at Location", "ST", max_length=200),
        "12": field("Administered Per (Time Unit)", "ST", max_length=20),
        "13": field("Administered Strength", "NM", max_length=20),
        "14": field("Administered Strength Units", "CE", max_length=250),
        "15": field("Substance Lot Number", "ST", repeating=True, max_length=20),
        "16": field("Substance Expiration Date", "TS", repeating=True, max_length=26),
        "17": field("Substance Manufacturer Name", "CE", repeating=True, max_length=250, table_id="0227"),
        "18": field("Substance/Treatment Refusal Reason", "CE", repeating=True, max_length=250),
        "19": field("Indication", "CE", repeating=True, max_length=250),
        "20": field("Completion Status", "ID", max_length=2, table_id="0322"),
        "21": field("Action Code - RXA", "ID", max_length=2, table_id="0323"),
        "22": field("System Entry Date/Time", "TS", max_length=26),
        "23": field("Administered Drug Strength Volume", "NM", max_length=5),
        "24": field("Administered Drug Strength Volume Units", "CWE", max_length=250),
        "25": field("Administered Barcode Identifier", "CWE", max_length=60),
        "26": field("Pharmacy Order Type", "ID", max_length=1, table_id="0480"),
    }


def get_rxd_fields():
    return {
        "1": field("Dispense Sub-ID Counter", "NM", required=True, max_length=4),
        "2": field("Dispense/Give Code", "CE", required=True, max_length=250, table_id="0292"),
        "3": field("Date/Time Dispensed", "TS", required=True, max_length=26),
        "4": field("Actual Dispense Amount", "NM", required=True, max_length=20),
        "5": field("Actual Dispense Units", "CE", max_length=250),
        "6": field("Actual Dosage Form", "CE", max_length=250),
        "7": field("Prescription Number", "ST", required=True, max_length=20),
        "8": field("Number of Refills Remaining", "NM", max_length=20),
        "9": field("Dispense Notes", "ST", repeating=True, max_length=200),
        "10": field("Dispensing Provider", "XCN", repeating=True, max_length=200),
        "11": field("Substitution Status", "ID", max_length=1, table_id="0167"),
        "12": field("Total Daily Dose", "CQ", max_length=10),
        "13": field("Dispense-to Location", "ST", max_length=200),
        "14": field("Needs Human Review", "ID", max_length=1, table_id="0136"),
        "15": field("Pharmacy/Treatment Supplier's Special Dispensing Instructions", "CE", repeating=True, max_length=250),
        "16": field("Actual Strength", "NM", max_length=20),
        "17": field("Actual Strength Unit", "CE", max_length=250),
        "18": field("Substance Lot Number", "ST", repeating=True, max_length=20),
        "19": field("Substance Expiration Date", "TS", repeating=True, max_length=26),
        "20": field("Substance Manufacturer Name", "CE", repeating=True, max_length=250, table_id="0227"),
        "21": field("Indication", "CE", repeating=True, max_length=250),
        "22": field("Dispense Package Size", "NM", max_length=20),
        "23": field("Dispense Package Size Unit", "CE", max_length=250),
        "24": field("Dispense Package Method", "ID", max_length=2, table_id="0321"),
        "25": field("Supplementary Code", "CE", repeating=True, max_length=250),
        "26": field("Initiating Location", "CE", max_length=250),
        "27": field("Packaging/Assembly Location", "CE", max_length=250),
        "28": field("Actual Drug Strength Volume", "NM", max_length=5),
        "29": field("Actual Drug Strength Volume Units", "CWE", max_length=250),
        "30": field("Dispense to Pharmacy", "CWE", max_length=180),
        "31": field("Dispense to Pharmacy Address", "XAD", max_length=250),
        "32": field("Pharmacy Order Type", "ID", max_length=1, table_id="0480"),
        "33": field("Dispense Type", "CWE", max_length=250, table_id="0484"),
    }


def get_rxe_fields():
    return {
        "1": field("Quantity/Timing", "TQ", max_length=200),
        "2": field("Give Code", "CE", required=True, max_length=250, table_id="0292"),
        "3": field("Give Amount - Minimum", "NM", required=True, max_length=20),
        "4": field("Give Amount - Maximum", "NM", max_length=20),
        "5": field("Give Units", "CE", required=True, max_length=250),
        "6": field("Give Dosage Form", "CE", max_length=250),
        "7": field("Provider's Administration Instructions", "CE", repeating=True, max_length=250),
        "8": field("Deliver-To Location", "ST", max_length=200),
        "9": field("Substitution Status", "ID", max_length=1, table_id="0167"),
        "10": field("Dispense Amount", "NM", max_length=20),
        "11": field("Dispense Units", "CE", max_length=250),
        "12": field("Number Of Refills", "NM", max_length=3),
        "13": field("Ordering Provider's DEA Number", "XCN", repeating=True, max_length=250),
        "14": field("Pharmacist/Treatment Supplier's Verifier ID", "XCN", repeating=True, max_length=250),
        "15": field("Prescription Number", "ST", max_length=20),
        "16": field("Number of Refills Remaining", "NM", max_length=20),
        "17": field("Number of Refills/Doses Dispensed", "NM", max_length=20),
        "18": field("D/T of Most Recent Refill or Dose Dispensed", "TS", max_length=26),
        "19": field("Total Daily Dose", "CQ", max_length=10),
        "20": field("Needs Human Review", "ID", max_length=1, table_id="0136"),
        "21": field("Pharmacy/Treatment Supplier's Special Dispensing Instructions", "CE", repeating=True, max_length=250),
        "22": field("Give Per (Time Unit)", "ST", max_length=20),
        "23": field("Give Rate Amount", "ST", max_length=6),
        "24": field("Give Rate Units", "CE", max_length=250),
        "25": field("Give Strength", "NM", max_length=20),
        "26": field("Give Strength Units", "CE", max_length=250),
        "27": field("Give Indication", "CE", repeating=True, max_length=250),
        "28": field("Dispense Package Size", "NM", max_length=20),
        "29": field("Dispense Package Size Unit", "CE", max_length=250),
        "30": field("Dispense Package Method", "ID", max_length=2, table_id="0321"),
        "31": field("Supplementary Code", "CE", repeating=True, max_length=250),
        "32": field("Original Order Date/Time", "TS", max_length=26),
        "33": field("Give Drug Strength Volume", "NM", max_length=5),
        "34": field("Give Drug Strength Volume Units", "CWE", max_length=250),
        "35": field("Controlled Substance Schedule", "CWE", max_length=60, table_id="0477"),
        "36": field("Formulary Status", "ID", max_length=1, table_id="0478"),
        "37": field("Pharmaceutical Substance Alternative", "CWE", repeating=True, max_length=60),
        "38": field("Pharmacy of Most Recent Fill", "CWE", max_length=250),
        "39": field("Initial Dispense Amount", "NM", max_length=250),
        "40": field("Dispensing Pharmacy", "CWE", max_length=250),
        "41": field("Dispensing Pharmacy Address", "XAD", max_length=250),
        "42": field("Deliver-to Patient Location", "PL", max_length=80),
        "43": field("Deliver-to Address", "XAD", max_length=250),
        "44": field("Pharmacy Order Type", "ID", max_length=1, table_id="0480"),
    }


def get_rxg_fields():
    return {
        "1": field("Give Sub-ID Counter", "NM", required=True, max_length=4),
        "2": field("Dispense Sub-ID Counter", "NM", max_length=4),
        "3": field("Quantity/Timing", "TQ", max_length=200),
        "4": field("Give Code", "CE", required=True, max_length=250, table_id="0292"),
        "5": field("Give Amount - Minimum", "NM", required=True, max_length=20),
        "6": field("Give Amount - Maximum", "NM", max_length=20),
        "7": field("Give Units", "CE", required=True, max_length=250),
        "8": field("Give Dosage Form", "CE", max_length=250),
        "9": field("Administration Notes", "CE", repeating=True, max_length=250),
        "10": field("Substitution Status", "ID", max_length=1, table_id="0167"),
        "11": field("Dispense-to Location", "ST", max_length=200),
        "12": field("Needs Human Review", "ID", max_length=1, table_id="0136"),
        "13": field("Pharmacy/Treatment Supplier's Special Administration Instructions", "CE", repeating=True, max_length=250),
        "14": field("Give Per (Time Unit)", "ST", max_length=20),
        "15": field("Give Rate Amount", "ST", max_length=6),
        "16": field("Give Rate Units", "CE", max_length=250),
        "17": field("Give Strength", "NM", max_length=20),
        "18": field("Give Strength Units", "CE", max_length=250),
        "19": field("Substance Lot Number", "ST", repeating=True, max_length=20),
        "20": field("Substance Expiration Date", "TS", repeating=True, max_length=26),
        "21": field("Substance Manufacturer Name", "CE", repeating=True, max_length=250, table_id="0227"),
        "22": field("Indication", "CE", repeating=True, max_length=250),
        "23": field("Give Drug Strength Volume", "NM", max_length=5),
        "24": field("Give Drug Strength Volume Units", "CWE", max_length=250),
        "25": field("Give Barcode Identifier", "CWE", max_length=60),
        "26": field("Pharmacy Order Type", "ID", max_length=1, table_id="0480"),
        "27": field("Dispense to Pharmacy", "CWE", max_length=180),
    }


def get_mfi_fields():
    return {
        "1": field("Master File Identifier", "CE", required=True, max_length=250, table_id="0175"),
        "2": field("Master File Application Identifier", "HD", repeating=True, max_length=180),
        "3": field("File-Level Event Code", "ID", required=True, max_length=3, table_id="0178"),
        "4": field("Entered Date/Time", "TS", max_length=26),
        "5": field("Effective Date/Time", "TS", max_length=26),
        "6": field("Response Level Code", "ID", required=True, max_length=2, table_id="0179"),
    }


def get_mfe_fields():
    return {
        "1": field("Record-Level Event Code", "ID", required=True, max_length=3, table_id="0180"),
        "2": field("MFN Control ID", "ST", max_length=20),
        "3": field("Effective Date/Time", "TS", max_length=26),
        "4": field("Primary Key Value - MFE", "Varies", required=True, repeating=True, max_length=200),
        "5": field("Primary Key Value Type", "ID", required=True, repeating=True, max_length=3, table_id="0355"),
    }


def get_spm_fields():
    return {
        "1": field("Set ID - SPM", "SI", max_length=4),
        "2": field("Specimen ID", "EIP", max_length=80),
        "3": field("Specimen Parent IDs", "EIP", repeating=True, max_length=80),
        "4": field("Specimen Type", "CWE", required=True, max_length=250, table_id="0487"),
        "5": field("Specimen Type Modifier", "CWE", repeating=True, max_length=250, table_id="0541"),
        "6": field("Specimen Additives", "CWE", repeating=True, max_length=250, table_id="0371"),
        "7": field("Specimen Collection Method", "CWE", max_length=250, table_id="0488"),
        "8": field("Specimen Source Site", "CWE", max_length=250),
        "9": field("Specimen Source Site Modifier", "CWE", repeating=True, max_length=250, table_id="0542"),
        "10": field("Specimen Collection Site", "CWE", max_length=250, table_id="0543"),
        "11": field("Specimen Role", "CWE", repeating=True, max_length=250, table_id="0369"),
        "12": field("Specimen Collection Amount", "CQ", max_length=20),
        "13": field("Grouped Specimen Count", "NM", max_length=6),
        "14": field("Specimen Description", "ST", repeating=True, max_length=250),
        "15": field("Specimen Handling Code", "CWE", repeating=True, max_length=250, table_id="0376"),
        "16": field("Specimen Risk Code", "CWE", repeating=True, max_length=250, table_id="0489"),
        "17": field("Specimen Collection Date/Time", "DR", max_length=26),
        "18": field("Specimen Received Date/Time", "TS", max_length=26),
        "19": field("Specimen Expiration Date/Time", "TS", max_length=26),
        "20": field("Specimen Availability", "ID", max_length=1, table_id="0136"),
        "21": field("Specimen Reject Reason", "CWE", repeating=True, max_length=250, table_id="0490"),
        "22": field("Specimen Quality", "CWE", max_length=250, table_id="0491"),
        "23": field("Specimen Appropriateness", "CWE", max_length=250, table_id="0492"),
        "24": field("Specimen Condition", "CWE", repeating=True, max_length=250, table_id="0493"),
        "25": field("Specimen Current Quantity", "CQ", max_length=20),
        "26": field("Number of Specimen Containers", "NM", max_length=4),
        "27": field("Container Type", "CWE", max_length=250),
        "28": field("Container Condition", "CWE", max_length=250, table_id="0544"),
        "29": field("Specimen Child Role", "CWE", max_length=250, table_id="0494"),
        "30": field("Accession ID", "CX", repeating=True, max_length=80),
    }


def get_rgs_fields():
    return {
        "1": field("Set ID - RGS", "SI", required=True, max_length=4),
        "2": field("Segment Action Code", "ID", max_length=3, table_id="0206"),
        "3": field("Resource Group ID", "CE", max_length=250),
    }


def get_ais_fields():
    return {
        "1": field("Set ID - AIS", "SI", required=True, max_length=4),
        "2": field("Segment Action Code", "ID", max_length=3, table_id="0206"),
        "3": field("Universal Service Identifier", "CE", required=True, max_length=250),
        "4": field("Start Date/Time", "TS", max_length=26),
        "5": field("Start Date/Time Offset", "NM", max_length=20),
        "6": field("Start Date/Time Offset Units", "CE", max_length=250),
        "7": field("Duration", "NM", max_length=20),
        "8": field("Duration Units", "CE", max_length=250),
        "9": field("Allow Substitution Code", "IS", max_length=10, table_id="0279"),
        "10": field("Filler Status Code", "CE", max_length=250, table_id="0278"),
        "11": field("Placer Supplemental Service Information", "CE", repeating=True, max_length=250, table_id="0411"),
        "12": field("Filler Supplemental Service Information", "CE", repeating=True, max_length=250, table_id="0411"),
    }


def get_nk1_fields():
    return {
        "1": field("Set ID - NK1", "SI", required=True, max_length=4),
        "2": field("Name", "XPN", repeating=True, max_length=250),
        "3": field("Relationship", "CE", max_length=250, table_id="0063"),
        "4": field("Address", "XAD", repeating=True, max_length=250),
        "5": field("Phone Number", "XTN", repeating=True, max_length=250),
        "6": field("Business Phone Number", "XTN", repeating=True, max_length=250),
        "7": field("Contact Role", "CE", max_length=250, table_id="0131"),
        "8": field("Start Date", "DT", max_length=8),
        "9": field("End Date", "DT", max_length=8),
        "10": field("Next of Kin / Associated Parties Job Title", "ST", max_length=60),
        "11": field("Next of Kin / Associated Parties Job Code/Class", "JCC", max_length=20),
        "12": field("Next of Kin / Associated Parties Employee Number", "CX", max_length=250),
        "13": field("Organization Name - NK1", "XON", repeating=True, max_length=250),
        "14": field("Marital Status", "CE", max_length=250, table_id="0002"),
        "15": field("Administrative Sex", "IS", max_length=1, table_id="0001"),
        "16": field("Date/Time of Birth", "TS", max_length=26),
        "17": field("Living Dependency", "IS", repeating=True, max_length=2, table_id="0223"),
        "18": field("Ambulatory Status", "IS", repeating=True, max_length=2, table_id="0009"),
        "19": field("Citizenship", "CE", repeating=True, max_length=250, table_id="0171"),
        "20": field("Primary Language", "CE", max_length=250, table_id="0296"),
        "21": field("Living Arrangement", "IS", max_length=2, table_id="0220"),
        "22": field("Publicity Code", "CE", max_length=250, table_id="0215"),
        "23": field("Protection Indicator", "ID", max_length=1, table_id="0136"),
        "24": field("Student Indicator", "IS", max_length=2, table_id="0231"),
        "25": field("Religion", "CE", max_length=250, table_id="0006"),
        "26": field("Mother's Maiden Name", "XPN", repeating=True, max_length=250),
        "27": field("Nationality", "CE", max_length=250, table_id="0212"),
        "28": field("Ethnic Group", "CE", repeating=True, max_length=250, table_id="0189"),
        "29": field("Contact Reason", "CE", repeating=True, max_length=250, table_id="0222"),
        "30": field("Contact Person's Name", "XPN", repeating=True, max_length=250),
        "31": field("Contact Person's Telephone Number", "XTN", repeating=True, max_length=250),
        "32": field("Contact Person's Address", "XAD", repeating=True, max_length=250),
        "33": field("Next of Kin/Associated Party's Identifiers", "CX", repeating=True, max_length=250),
        "34": field("Job Status", "IS", max_length=2, table_id="0311"),
        "35": field("Race", "CE", repeating=True, max_length=250, table_id="0005"),
        "36": field("Handicap", "IS", max_length=2, table_id="0295"),
        "37": field("Contact Person Social Security Number", "ST", max_length=16),
        "38": field("Next of Kin Birth Place", "ST", max_length=250),
        "39": field("VIP Indicator", "IS", max_length=2, table_id="0099"),
    }


def get_pv2_fields():
    return {
        "1": field("Prior Pending Location", "PL", max_length=80),
        "2": field("Accommodation Code", "CE", max_length=250, table_id="0129"),
        "3": field("Admit Reason", "CE", max_length=250),
        "4": field("Transfer Reason", "CE", max_length=250),
        "5": field("Patient Valuables", "ST", repeating=True, max_length=25),
        "6": field("Patient Valuables Location", "ST", max_length=25),
        "7": field("Visit User Code", "IS", repeating=True, max_length=2, table_id="0130"),
        "8": field("Expected Admit Date/Time", "TS", max_length=26),
        "9": field("Expected Discharge Date/Time", "TS", max_length=26),
        "10": field("Estimated Length of Inpatient Stay", "NM", max_length=3),
        "11": field("Actual Length of Inpatient Stay", "NM", max_length=3),
        "12": field("Visit Description", "ST", max_length=50),
        "13": field("Referral Source Code", "XCN", repeating=True, max_length=250),
        "14": field("Previous Service Date", "DT", max_length=8),
        "15": field("Employment Illness Related Indicator", "ID", max_length=1, table_id="0136"),
        "16": field("Purge Status Code", "IS", max_length=1, table_id="0213"),
        "17": field("Purge Status Date", "DT", max_length=8),
        "18": field("Special Program Code", "IS", max_length=2, table_id="0214"),
        "19": field("Retention Indicator", "ID", max_length=1, table_id="0136"),
        "20": field("Expected Number of Insurance Plans", "NM", max_length=1),
        "21": field("Visit Publicity Code", "IS", max_length=1, table_id="0215"),
        "22": field("Visit Protection Indicator", "ID", max_length=1, table_id="0136"),
        "23": field("Clinic Organization Name", "XON", repeating=True, max_length=250),
        "24": field("Patient Status Code", "IS", max_length=2, table_id="0216"),
        "25": field("Visit Priority Code", "IS", max_length=1, table_id="0217"),
        "26": field("Previous Treatment Date", "DT", max_length=8),
        "27": field("Expected Discharge Disposition", "IS", max_length=2, table_id="0112"),
        "28": field("Signature on File Date", "DT", max_length=8),
        "29": field("First Similar Illness Date", "DT", max_length=8),
        "30": field("Patient Charge Adjustment Code", "CE", max_length=250, table_id="0218"),
        "31": field("Recurring Service Code", "IS", max_length=2, table_id="0219"),
        "32": field("Billing Media Code", "ID", max_length=1, table_id="0136"),
        "33": field("Expected Surgery Date and Time", "TS", max_length=26),
        "34": field("Military Partnership Code", "ID", max_length=1, table_id="0136"),
        "35": field("Military Non-Availability Code", "ID", max_length=1, table_id="0136"),
        "36": field("Newborn Baby Indicator", "ID", max_length=1, table_id="0136"),
        "37": field("Baby Detained Indicator", "ID", max_length=1, table_id="0136"),
        "38": field("Mode of Arrival Code", "CE", max_length=250, table_id="0430"),
        "39": field("Recreational Drug Use Code", "CE", repeating=True, max_length=250, table_id="0431"),
        "40": field("Admission Level of Care Code", "CE", max_length=250, table_id="0432"),
        "41": field("Precaution Code", "CE", repeating=True, max_length=250, table_id="0433"),
        "42": field("Patient Condition Code", "CE", max_length=250, table_id="0434"),
        "43": field("Living Will Code", "IS", max_length=2, table_id="0315"),
        "44": field("Organ Donor Code", "IS", max_length=2, table_id="0316"),
        "45": field("Advance Directive Code", "CE", repeating=True, max_length=250, table_id="0435"),
        "46": field("Patient Status Effective Date", "DT", max_length=8),
        "47": field("Expected LOA Return Date/Time", "TS", max_length=26),
    }


def get_in1_fields():
    return {
        "1": field("Set ID - IN1", "SI", required=True, max_length=4),
        "2": field("Insurance Plan ID", "CE", required=True, max_length=250, table_id="0072"),
        "3": field("Insurance Company ID", "CX", required=True, repeating=True, max_length=250),
        "4": field("Insurance Company Name", "XON", repeating=True, max_length=250),
        "5": field("Insurance Company Address", "XAD", repeating=True, max_length=250),
        "6": field("Insurance Co Contact Person", "XPN", repeating=True, max_length=250),
        "7": field("Insurance Co Phone Number", "XTN", repeating=True, max_length=250),
        "8": field("Group Number", "ST", max_length=12),
        "9": field("Group Name", "XON", repeating=True, max_length=250),
        "10": field("Insured's Group Emp ID", "CX", repeating=True, max_length=250),
        "11": field("Insured's Group Emp Name", "XON", repeating=True, max_length=250),
        "12": field("Plan Effective Date", "DT", max_length=8),
        "13": field("Plan Expiration Date", "DT", max_length=8),
        "14": field("Authorization Information", "ST", max_length=239),
        "15": field("Plan Type", "IS", max_length=3, table_id="0086"),
        "16": field("Name Of Insured", "XPN", repeating=True, max_length=250),
        "17": field("Insured's Relationship To Patient", "CE", max_length=250, table_id="0063"),
        "18": field("Insured's Date Of Birth", "TS", max_length=26),
        "19": field("Insured's Address", "XAD", repeating=True, max_length=250),
        "20": field("Assignment Of Benefits", "IS", max_length=2, table_id="0135"),
        "21": field("Coordination Of Benefits", "IS", max_length=2, table_id="0173"),
        "22": field("Coord Of Ben. Priority", "ST", max_length=2),
        "23": field("Notice Of Admission Flag", "ID", max_length=1, table_id="0136"),
        "24": field("Notice Of Admission Date", "DT", max_length=8),
        "25": field("Report Of Eligibility Flag", "ID", max_length=1, table_id="0136"),
        "26": field("Report Of Eligibility Date", "DT", max_length=8),
        "27": field("Release Information Code", "IS", max_length=2, table_id="0093"),
        "28": field("Pre-Admit Cert (PAC)", "ST", max_length=15),
        "29": field("Verification Date/Time", "TS", max_length=26),
        "30": field("Verification By", "XCN", repeating=True, max_length=250),
        "31": field("Type Of Agreement Code", "IS", max_length=2, table_id="0098"),
        "32": field("Billing Status", "IS", max_length=2, table_id="0022"),
        "33": field("Lifetime Reserve Days", "NM", max_length=4),
        "34": field("Delay Before L.R. Day", "NM", max_length=4),
        "35": field("Company Plan Code", "IS", max_length=8, table_id="0042"),
        "36": field("Policy Number", "ST", max_length=15),
        "37": field("Policy Deductible", "CP", max_length=12),
        "38": field("Policy Limit - Amount", "CP", max_length=12),
        "39": field("Policy Limit - Days", "NM", max_length=4),
        "40": field("Room Rate - Semi-Private", "CP", max_length=12),
        "41": field("Room Rate - Private", "CP", max_length=12),
        "42": field("Insured's Employment Status", "CE", max_length=250, table_id="0066"),
        "43": field("Insured's Administrative Sex", "IS", max_length=1, table_id="0001"),
        "44": field("Insured's Employer's Address", "XAD", repeating=True, max_length=250),
        "45": field("Verification Status", "ST", max_length=2),
        "46": field("Prior Insurance Plan ID", "IS", max_length=8, table_id="0072"),
        "47": field("Coverage Type", "IS", max_length=3, table_id="0309"),
        "48": field("Handicap", "IS", max_length=2, table_id="0295"),
        "49": field("Insured's ID Number", "CX", repeating=True, max_length=250),
    }


def get_in2_fields():
    return {
        "1": field("Insured's Employee ID", "CX", repeating=True, max_length=250),
        "2": field("Insured's Social Security Number", "ST", max_length=11),
        "3": field("Insured's Employer's Name and ID", "XCN", repeating=True, max_length=250),
        "4": field("Employer Information Data", "IS", max_length=1, table_id="0139"),
        "5": field("Mail Claim Party", "IS", repeating=True, max_length=1, table_id="0137"),
        "6": field("Medicare Health Ins Card Number", "ST", max_length=15),
        "7": field("Medicaid Case Name", "XPN", repeating=True, max_length=250),
        "8": field("Medicaid Case Number", "ST", max_length=15),
        "9": field("Military Sponsor Name", "XPN", repeating=True, max_length=250),
        "10": field("Military ID Number", "ST", max_length=20),
        "11": field("Dependent Of Military Recipient", "CE", max_length=250, table_id="0342"),
        "12": field("Military Organization", "ST", max_length=25),
        "13": field("Military Station", "ST", max_length=25),
        "14": field("Military Service", "IS", max_length=14, table_id="0140"),
        "15": field("Military Rank/Grade", "IS", max_length=2, table_id="0141"),
        "16": field("Military Status", "IS", max_length=3, table_id="0142"),
        "17": field("Military Retire Date", "DT", max_length=8),
        "18": field("Military Non-Avail Cert On File", "ID", max_length=1, table_id="0136"),
        "19": field("Baby Coverage", "ID", max_length=1, table_id="0136"),
        "20": field("Combine Baby Bill", "ID", max_length=1, table_id="0136"),
        "21": field("Blood Deductible", "ST", max_length=1),
        "22": field("Special Coverage Approval Name", "XPN", repeating=True, max_length=250),
        "23": field("Special Coverage Approval Title", "ST", max_length=30),
        "24": field("Non-Covered Insurance Code", "IS", repeating=True, max_length=8, table_id="0143"),
        "25": field("Payor ID", "CX", repeating=True, max_length=250),
        "26": field("Payor Subscriber ID", "CX", repeating=True, max_length=250),
        "27": field("Eligibility Source", "IS", max_length=1, table_id="0144"),
        "28": field("Room Coverage Type/Amount", "ST", repeating=True, max_length=25),
        "29": field("Policy Type/Amount", "ST", repeating=True, max_length=25),
        "30": field("Daily Deductible", "MO", max_length=25),
    }


def get_dg1_fields():
    return {
        "1": field("Set ID - DG1", "SI", required=True, max_length=4),
        "2": field("Diagnosis Coding Method", "ID", max_length=2, table_id="0053"),
        "3": field("Diagnosis Code - DG1", "CE", max_length=250, table_id="0051"),
        "4": field("Diagnosis Description", "ST", max_length=40),
        "5": field("Diagnosis Date/Time", "TS", max_length=26),
        "6": field("Diagnosis Type", "IS", required=True, max_length=2, table_id="0052"),
        "7": field("Major Diagnostic Category", "CE", max_length=250, table_id="0118"),
        "8": field("Diagnostic Related Group", "CE", max_length=250, table_id="0055"),
        "9": field("DRG Approval Indicator", "ID", max_length=1, table_id="0136"),
        "10": field("DRG Grouper Review Code", "IS", max_length=2, table_id="0056"),
        "11": field("Outlier Type", "CE", max_length=250, table_id="0083"),
        "12": field("Outlier Days", "NM", max_length=3),
        "13": field("Outlier Cost", "CP", max_length=12),
        "14": field("Grouper Version And Type", "ST", max_length=4),
        "15": field("Diagnosis Priority", "ID", max_length=2, table_id="0359"),
        "16": field("Diagnosing Clinician", "XCN", repeating=True, max_length=250),
        "17": field("Diagnosis Classification", "IS", max_length=3, table_id="0228"),
        "18": field("Confidential Indicator", "ID", max_length=1, table_id="0136"),
        "19": field("Attestation Date/Time", "TS", max_length=26),
        "20": field("Diagnosis Identifier", "EI", max_length=427),
        "21": field("Diagnosis Action Code", "ID", max_length=1, table_id="0206"),
    }


def get_al1_fields():
    return {
        "1": field("Set ID - AL1", "SI", required=True, max_length=4),
        "2": field("Allergen Type Code", "CE", max_length=250, table_id="0127"),
        "3": field("Allergen Code/Mnemonic/Description", "CE", required=True, max_length=250),
        "4": field("Allergy Severity Code", "CE", max_length=250, table_id="0128"),
        "5": field("Allergy Reaction Code", "ST", repeating=True, max_length=15),
        "6": field("Identification Date", "DT", max_length=8),
    }


def get_gt1_fields():
    return {
        "1": field("Set ID - GT1", "SI", required=True, max_length=4),
        "2": field("Guarantor Number", "CX", repeating=True, max_length=250),
        "3": field("Guarantor Name", "XPN", required=True, repeating=True, max_length=250),
        "4": field("Guarantor Spouse Name", "XPN", repeating=True, max_length=250),
        "5": field("Guarantor Address", "XAD", repeating=True, max_length=250),
        "6": field("Guarantor Ph Num - Home", "XTN", repeating=True, max_length=250),
        "7": field("Guarantor Ph Num - Business", "XTN", repeating=True, max_length=250),
        "8": field("Guarantor Date/Time Of Birth", "TS", max_length=26),
        "9": field("Guarantor Administrative Sex", "IS", max_length=1, table_id="0001"),
        "10": field("Guarantor Type", "IS", max_length=2, table_id="0068"),
        "11": field("Guarantor Relationship", "CE", max_length=250, table_id="0063"),
        "12": field("Guarantor SSN", "ST", max_length=11),
        "13": field("Guarantor Date - Begin", "DT", max_length=8),
        "14": field("Guarantor Date - End", "DT", max_length=8),
        "15": field("Guarantor Priority", "NM", max_length=2),
        "16": field("Guarantor Employer Name", "XPN", repeating=True, max_length=250),
        "17": field("Guarantor Employer Address", "XAD", repeating=True, max_length=250),
        "18": field("Guarantor Employer Phone Number", "XTN", repeating=True, max_length=250),
        "19": field("Guarantor Employee ID Number", "CX", repeating=True, max_length=250),
        "20": field("Guarantor Employment Status", "IS", max_length=2, table_id="0066"),
        "21": field("Guarantor Organization Name", "XON", repeating=True, max_length=250),
    }


def get_nte_fields():
    return {
        "1": field("Set ID - NTE", "SI", max_length=4),
        "2": field("Source of Comment", "ID", max_length=8, table_id="0105"),
        "3": field("Comment", "FT", repeating=True, max_length=65536),
        "4": field("Comment Type", "CE", max_length=250, table_id="0364"),
    }


def get_pr1_fields():
    return {
        "1": field("Set ID - PR1", "SI", required=True, max_length=4),
        "2": field("Procedure Coding Method", "IS", max_length=3, table_id="0089"),
        "3": field("Procedure Code", "CE", required=True, max_length=250, table_id="0088"),
        "4": field("Procedure Description", "ST", max_length=40),
        "5": field("Procedure Date/Time", "TS", required=True, max_length=26),
        "6": field("Procedure Functional Type", "IS", max_length=2, table_id="0230"),
        "7": field("Procedure Minutes", "NM", max_length=4),
        "8": field("Anesthesiologist", "XCN", repeating=True, max_length=250),
        "9": field("Anesthesia Code", "IS", max_length=2, table_id="0019"),
        "10": field("Anesthesia Minutes", "NM", max_length=4),
        "11": field("Surgeon", "XCN", repeating=True, max_length=250),
        "12": field("Procedure Practitioner", "XCN", repeating=True, max_length=250),
        "13": field("Consent Code", "CE", max_length=250, table_id="0059"),
        "14": field("Procedure Priority", "ID", max_length=2, table_id="0418"),
        "15": field("Associated Diagnosis Code", "CE", max_length=250, table_id="0051"),
        "16": field("Procedure Code Modifier", "CE", repeating=True, max_length=250, table_id="0340"),
        "17": field("Procedure DRG Type", "IS", max_length=20, table_id="0416"),
        "18": field("Tissue Type Code", "CE", repeating=True, max_length=250, table_id="0417"),
        "19": field("Procedure Identifier", "EI", max_length=427),
        "20": field("Procedure Action Code", "ID", max_length=1, table_id="0206"),
    }


def get_pd1_fields():
    return {
        "1": field("Living Dependency", "IS", repeating=True, max_length=2, table_id="0223"),
        "2": field("Living Arrangement", "IS", max_length=2, table_id="0220"),
        "3": field("Patient Primary Facility", "XON", repeating=True, max_length=250, table_id="0204"),
        "4": field("Patient Primary Care Provider Name & ID No.", "XCN", repeating=True, max_length=250),
        "5": field("Student Indicator", "IS", max_length=2, table_id="0231"),
        "6": field("Handicap", "IS", max_length=2, table_id="0295"),
        "7": field("Living Will Code", "IS", max_length=2, table_id="0315"),
        "8": field("Organ Donor Code", "IS", max_length=2, table_id="0316"),
        "9": field("Separate Bill", "ID", max_length=1, table_id="0136"),
        "10": field("Duplicate Patient", "CX", repeating=True, max_length=250),
        "11": field("Publicity Code", "CE", max_length=250, table_id="0215"),
        "12": field("Protection Indicator", "ID", max_length=1, table_id="0136"),
        "13": field("Protection Indicator Effective Date", "DT", max_length=8),
        "14": field("Place of Worship", "XON", repeating=True, max_length=250),
        "15": field("Advance Directive Code", "CE", repeating=True, max_length=250, table_id="0435"),
        "16": field("Immunization Registry Status", "IS", max_length=1, table_id="0441"),
        "17": field("Immunization Registry Status Effective Date", "DT", max_length=8),
        "18": field("Publicity Code Effective Date", "DT", max_length=8),
        "19": field("Military Branch", "IS", max_length=5, table_id="0140"),
        "20": field("Military Rank/Grade", "IS", max_length=2, table_id="0141"),
        "21": field("Military Status", "IS", max_length=3, table_id="0142"),
    }


def get_db1_fields():
    return {
        "1": field("Set ID - DB1", "SI", required=True, max_length=4),
        "2": field("Disabled Person Code", "IS", max_length=2, table_id="0334"),
        "3": field("Disabled Person Identifier", "CX", repeating=True, max_length=250),
        "4": field("Disabled Indicator", "ID", max_length=1, table_id="0136"),
        "5": field("Disability Start Date", "DT", max_length=8),
        "6": field("Disability End Date", "DT", max_length=8),
        "7": field("Disability Return to Work Date", "DT", max_length=8),
        "8": field("Disability Unable to Work Date", "DT", max_length=8),
    }


def get_drg_fields():
    return {
        "1": field("Diagnostic Related Group", "CE", max_length=250, table_id="0055"),
        "2": field("DRG Assigned Date/Time", "TS", max_length=26),
        "3": field("DRG Approval Indicator", "ID", max_length=1, table_id="0136"),
        "4": field("DRG Grouper Review Code", "IS", max_length=2, table_id="0056"),
        "5": field("Outlier Type", "CE", max_length=250, table_id="0083"),
        "6": field("Outlier Days", "NM", max_length=3),
        "7": field("Outlier Cost", "CP", max_length=12),
        "8": field("DRG Payor", "IS", max_length=1, table_id="0229"),
        "9": field("Outlier Reimbursement", "CP", max_length=9),
        "10": field("Confidential Indicator", "ID", max_length=1, table_id="0136"),
        "11": field("DRG Transfer Type", "IS", max_length=21, table_id="0415"),
    }


def get_acc_fields():
    return {
        "1": field("Accident Date/Time", "TS", max_length=26),
        "2": field("Accident Code", "CE", max_length=250, table_id="0050"),
        "3": field("Accident Location", "ST", max_length=25),
        "4": field("Auto Accident State", "CE", max_length=250, table_id="0347"),
        "5": field("Accident Job Related Indicator", "ID", max_length=1, table_id="0136"),
        "6": field("Accident Death Indicator", "ID", max_length=12, table_id="0136"),
    }


def get_rxr_fields():
    return {
        "1": field("Route", "CE", required=True, max_length=250, table_id="0162"),
        "2": field("Administration Site", "CWE", max_length=250, table_id="0550"),
        "3": field("Administration Device", "CE", max_length=250, table_id="0164"),
        "4": field("Administration Method", "CWE", max_length=250, table_id="0165"),
        "5": field("Routing Instruction", "CE", max_length=250),
        "6": field("Administration Site Modifier", "CWE", max_length=250, table_id="0495"),
    }


def get_rxc_fields():
    return {
        "1": field("RX Component Type", "ID", required=True, max_length=1, table_id="0166"),
        "2": field("Component Code", "CE", required=True, max_length=250),
        "3": field("Component Amount", "NM", required=True, max_length=20),
        "4": field("Component Units", "CE", required=True, max_length=250),
        "5": field("Component Strength", "NM", max_length=20),
        "6": field("Component Strength Units", "CE", max_length=250),
        "7": field("Supplementary Code", "CE", repeating=True, max_length=250),
        "8": field("Component Drug Strength Volume", "NM", max_length=5),
        "9": field("Component Drug Strength Volume Units", "CWE", max_length=250),
    }


def get_rol_fields():
    return {
        "1": field("Role Instance ID", "EI", max_length=60),
        "2": field("Action Code", "ID", required=True, max_length=2, table_id="0287"),
        "3": field("Role-ROL", "CE", required=True, max_length=250, table_id="0443"),
        "4": field("Role Person", "XCN", required=True, repeating=True, max_length=250),
        "5": field("Role Begin Date/Time", "TS", max_length=26),
        "6": field("Role End Date/Time", "TS", max_length=26),
        "7": field("Role Duration", "CE", max_length=250),
        "8": field("Role Action Reason", "CE", max_length=250),
        "9": field("Provider Type", "CE", repeating=True, max_length=250),
        "10": field("Organization Unit Type", "CE", max_length=250, table_id="0406"),
        "11": field("Office/Home Address/Birthplace", "XAD", repeating=True, max_length=250),
        "12": field("Phone", "XTN", repeating=True, max_length=250),
    }


def get_sft_fields():
    return {
        "1": field("Software Vendor Organization", "XON", required=True, max_length=567),
        "2": field("Software Certified Version or Release Number", "ST", required=True, max_length=15),
        "3": field("Software Product Name", "ST", required=True, max_length=20),
        "4": field("Software Binary ID", "ST", required=True, max_length=20),
        "5": field("Software Product Information", "TX", max_length=1024),
        "6": field("Software Install Date", "TS", max_length=26),
    }


def get_tq1_fields():
    return {
        "1": field("Set ID - TQ1", "SI", max_length=4),
        "2": field("Quantity", "CQ", max_length=20),
        "3": field("Repeat Pattern", "ST", repeating=True, max_length=540),
        "4": field("Explicit Time", "TM", repeating=True, max_length=20),
        "5": field("Relative Time and Units", "CQ", repeating=True, max_length=20),
        "6": field("Service Duration", "CQ", max_length=20),
        "7": field("Start date/time", "TS", max_length=26),
        "8": field("End date/time", "TS", max_length=26),
        "9": field("Priority", "CWE", repeating=True, max_length=250, table_id="0485"),
        "10": field("Condition text", "TX", max_length=250),
        "11": field("Text instruction", "TX", max_length=250),
        "12": field("Conjunction", "ID", max_length=10, table_id="0472"),
        "13": field("Occurrence duration", "CQ", max_length=20),
        "14": field("Total occurrences", "NM", max_length=10),
    }


def get_tq2_fields():
    return {
        "1": field("Set ID - TQ2", "SI", max_length=4),
        "2": field("Sequence/Results Flag", "ID", max_length=1, table_id="0503"),
        "3": field("Related Placer Number", "EI", repeating=True, max_length=22),
        "4": field("Related Filler Number", "EI", repeating=True, max_length=22),
        "5": field("Related Placer Group Number", "EI", repeating=True, max_length=22),
        "6": field("Sequence Condition Code", "ID", max_length=2, table_id="0504"),
        "7": field("Cyclic Entry/Exit Indicator", "ID", max_length=1, table_id="0505"),
        "8": field("Sequence Condition Time Interval", "CQ", max_length=20),
        "9": field("Cyclic Group Maximum Number of Repeats", "NM", max_length=10),
        "10": field("Special Service Request Relationship", "ID", max_length=1, table_id="0506"),
    }


def get_aig_fields():
    return {
        "1": field("Set ID - AIG", "SI", required=True, max_length=4),
        "2": field("Segment Action Code", "ID", max_length=3, table_id="0206"),
        "3": field("Resource ID", "CE", max_length=250),
        "4": field("Resource Type", "CE", required=True, max_length=250),
        "5": field("Resource Group", "CE", repeating=True, max_length=250),
        "6": field("Resource Quantity", "NM", max_length=5),
        "7": field("Resource Quantity Units", "CE", max_length=250),
        "8": field("Start Date/Time", "TS", max_length=26),
        "9": field("Start Date/Time Offset", "NM", max_length=20),
        "10": field("Start Date/Time Offset Units", "CE", max_length=250),
        "11": field("Duration", "NM", max_length=20),
        "12": field("Duration Units", "CE", max_length=250),
        "13": field("Allow Substitution Code", "IS", max_length=10, table_id="0279"),
        "14": field("Filler Status Code", "CE", max_length=250, table_id="0278"),
    }


def get_ail_fields():
    return {
        "1": field("Set ID - AIL", "SI", required=True, max_length=4),
        "2": field("Segment Action Code", "ID", max_length=3, table_id="0206"),
        "3": field("Location Resource ID", "PL", repeating=True, max_length=80),
        "4": field("Location Type - AIL", "CE", max_length=250, table_id="0305"),
        "5": field("Location Group", "CE", max_length=250),
        "6": field("Start Date/Time", "TS", max_length=26),
        "7": field("Start Date/Time Offset", "NM", max_length=20),
        "8": field("Start Date/Time Offset Units", "CE", max_length=250),
        "9": field("Duration", "NM", max_length=20),
        "10": field("Duration Units", "CE", max_length=250),
        "11": field("Allow Substitution Code", "IS", max_length=10, table_id="0279"),
        "12": field("Filler Status Code", "CE", max_length=250, table_id="0278"),
    }


def get_aip_fields():
    return {
        "1": field("Set ID - AIP", "SI", required=True, max_length=4),
        "2": field("Segment Action Code", "ID", max_length=3, table_id="0206"),
        "3": field("Personnel Resource ID", "XCN", repeating=True, max_length=250),
        "4": field("Resource Type", "CE", max_length=250, table_id="0182"),
        "5": field("Resource Group", "CE", max_length=250),
        "6": field("Start Date/Time", "TS", max_length=26),
        "7": field("Start Date/Time Offset", "NM", max_length=20),
        "8": field("Start Date/Time Offset Units", "CE", max_length=250),
        "9": field("Duration", "NM", max_length=20),
        "10": field("Duration Units", "CE", max_length=250),
        "11": field("Allow Substitution Code", "IS", max_length=10, table_id="0279"),
        "12": field("Filler Status Code", "CE", max_length=250, table_id="0278"),
    }


def get_blg_fields():
    return {
        "1": field("When to Charge", "ST", max_length=40, table_id="0100"),
        "2": field("Charge Type", "ID", max_length=50, table_id="0122"),
        "3": field("Account ID", "CX", max_length=100),
        "4": field("Charge Type Reason", "CWE", max_length=60, table_id="0475"),
    }


# =============================================================================
# SEGMENT METADATA (name, required, repeating) per message type
# =============================================================================

# Base segment definitions with metadata
SEGMENT_META = {
    "MSH": {"name": "Message Header", "required": True, "repeating": False},
    "EVN": {"name": "Event Type", "required": True, "repeating": False},
    "PID": {"name": "Patient Identification", "required": True, "repeating": False},
    "PV1": {"name": "Patient Visit", "required": True, "repeating": False},
    "OBR": {"name": "Observation Request", "required": True, "repeating": True},
    "OBX": {"name": "Observation/Result", "required": False, "repeating": True},
    "ORC": {"name": "Common Order", "required": True, "repeating": True},
    "MSA": {"name": "Message Acknowledgment", "required": True, "repeating": False},
    "ERR": {"name": "Error", "required": False, "repeating": True},
    "MRG": {"name": "Merge Patient Information", "required": True, "repeating": False},
    "FT1": {"name": "Financial Transaction", "required": False, "repeating": True},
    "TXA": {"name": "Transcription Document Header", "required": True, "repeating": False},
    "SCH": {"name": "Scheduling Activity Information", "required": True, "repeating": False},
    "QRD": {"name": "Original-Style Query Definition", "required": True, "repeating": False},
    "QRF": {"name": "Original-Style Query Filter", "required": False, "repeating": False},
    "RXA": {"name": "Pharmacy/Treatment Administration", "required": True, "repeating": True},
    "RXD": {"name": "Pharmacy/Treatment Dispense", "required": True, "repeating": False},
    "RXE": {"name": "Pharmacy/Treatment Encoded Order", "required": True, "repeating": False},
    "RXG": {"name": "Pharmacy/Treatment Give", "required": True, "repeating": True},
    "MFI": {"name": "Master File Identification", "required": True, "repeating": False},
    "MFE": {"name": "Master File Entry", "required": True, "repeating": True},
    "SPM": {"name": "Specimen", "required": False, "repeating": True},
    "RGS": {"name": "Resource Group", "required": True, "repeating": True},
    "AIS": {"name": "Appointment Information - Service", "required": False, "repeating": True},
    "NK1": {"name": "Next of Kin / Associated Parties", "required": False, "repeating": True},
    "PV2": {"name": "Patient Visit - Additional Information", "required": False, "repeating": False},
    "IN1": {"name": "Insurance", "required": False, "repeating": True},
    "IN2": {"name": "Insurance Additional Information", "required": False, "repeating": True},
    "DG1": {"name": "Diagnosis", "required": False, "repeating": True},
    "AL1": {"name": "Patient Allergy Information", "required": False, "repeating": True},
    "GT1": {"name": "Guarantor", "required": False, "repeating": True},
    "NTE": {"name": "Notes and Comments", "required": False, "repeating": True},
    "PR1": {"name": "Procedures", "required": False, "repeating": True},
    "ROL": {"name": "Role", "required": False, "repeating": True},
    "PD1": {"name": "Patient Additional Demographic", "required": False, "repeating": False},
    "DB1": {"name": "Disability", "required": False, "repeating": True},
    "DRG": {"name": "Diagnosis Related Group", "required": False, "repeating": False},
    "ACC": {"name": "Accident", "required": False, "repeating": False},
    "RXR": {"name": "Pharmacy/Treatment Route", "required": False, "repeating": True},
    "RXC": {"name": "Pharmacy/Treatment Component Order", "required": False, "repeating": True},
    "SFT": {"name": "Software Segment", "required": False, "repeating": True},
    "TQ1": {"name": "Timing/Quantity", "required": False, "repeating": True},
    "TQ2": {"name": "Timing/Quantity Relationship", "required": False, "repeating": True},
    "AIG": {"name": "Appointment Information - General Resource", "required": False, "repeating": True},
    "AIL": {"name": "Appointment Information - Location Resource", "required": False, "repeating": True},
    "AIP": {"name": "Appointment Information - Personnel Resource", "required": False, "repeating": True},
    "BLG": {"name": "Billing", "required": False, "repeating": False},
}

# Segment field generators
SEGMENT_FIELD_GENERATORS = {
    "MSH": get_msh_fields,
    "EVN": get_evn_fields,
    "PID": get_pid_fields,
    "PV1": get_pv1_fields,
    "OBR": get_obr_fields,
    "OBX": get_obx_fields,
    "ORC": get_orc_fields,
    "MSA": get_msa_fields,
    "ERR": get_err_fields,
    "MRG": get_mrg_fields,
    "FT1": get_ft1_fields,
    "TXA": get_txa_fields,
    "SCH": get_sch_fields,
    "QRD": get_qrd_fields,
    "QRF": get_qrf_fields,
    "RXA": get_rxa_fields,
    "RXD": get_rxd_fields,
    "RXE": get_rxe_fields,
    "RXG": get_rxg_fields,
    "MFI": get_mfi_fields,
    "MFE": get_mfe_fields,
    "SPM": get_spm_fields,
    "RGS": get_rgs_fields,
    "AIS": get_ais_fields,
    "NK1": get_nk1_fields,
    "PV2": get_pv2_fields,
    "IN1": get_in1_fields,
    "IN2": get_in2_fields,
    "DG1": get_dg1_fields,
    "AL1": get_al1_fields,
    "GT1": get_gt1_fields,
    "NTE": get_nte_fields,
    "PR1": get_pr1_fields,
    "ROL": get_rol_fields,
    "PD1": get_pd1_fields,
    "DB1": get_db1_fields,
    "DRG": get_drg_fields,
    "ACC": get_acc_fields,
    "RXR": get_rxr_fields,
    "RXC": get_rxc_fields,
    "SFT": get_sft_fields,
    "TQ1": get_tq1_fields,
    "TQ2": get_tq2_fields,
    "AIG": get_aig_fields,
    "AIL": get_ail_fields,
    "AIP": get_aip_fields,
    "BLG": get_blg_fields,
}

# =============================================================================
# MESSAGE DEFINITIONS - which segments each message type contains
# =============================================================================

MESSAGE_DEFINITIONS = {
    "ACK": {
        "description": "General Acknowledgment",
        "segments": ["MSH", "MSA", "ERR"],
        "overrides": {
            "EVN": {"required": False},
        }
    },
    "ADT_A01": {"description": "Admit/Visit Notification", "segments": ["MSH", "SFT", "EVN", "PID", "PD1", "ROL", "NK1", "PV1", "PV2", "ROL", "DB1", "OBX", "AL1", "DG1", "DRG", "PR1", "GT1", "IN1", "IN2", "ACC", "NTE"]},
    "ADT_A02": {"description": "Transfer a Patient", "segments": ["MSH", "SFT", "EVN", "PID", "PD1", "ROL", "PV1", "PV2", "DB1", "OBX"]},
    "ADT_A03": {"description": "Discharge/End Visit", "segments": ["MSH", "SFT", "EVN", "PID", "PD1", "ROL", "NK1", "PV1", "PV2", "DB1", "AL1", "DG1", "DRG", "PR1", "OBX", "GT1", "IN1", "IN2", "ACC"]},
    "ADT_A04": {"description": "Register a Patient", "segments": ["MSH", "SFT", "EVN", "PID", "PD1", "ROL", "NK1", "PV1", "PV2", "DB1", "OBX", "AL1", "DG1", "DRG", "PR1", "GT1", "IN1", "IN2", "ACC", "NTE"]},
    "ADT_A05": {"description": "Pre-Admit a Patient", "segments": ["MSH", "SFT", "EVN", "PID", "PD1", "ROL", "NK1", "PV1", "PV2", "DB1", "OBX", "AL1", "DG1", "DRG", "PR1", "GT1", "IN1", "IN2", "ACC"]},
    "ADT_A06": {"description": "Change an Outpatient to an Inpatient", "segments": ["MSH", "SFT", "EVN", "PID", "PD1", "ROL", "MRG", "NK1", "PV1", "PV2", "DB1", "OBX", "AL1", "DG1", "DRG", "PR1", "GT1", "IN1", "IN2", "ACC"]},
    "ADT_A07": {"description": "Change an Inpatient to an Outpatient", "segments": ["MSH", "SFT", "EVN", "PID", "PD1", "ROL", "MRG", "NK1", "PV1", "PV2", "DB1", "OBX", "AL1", "DG1", "DRG", "PR1", "GT1", "IN1", "IN2", "ACC"]},
    "ADT_A08": {"description": "Update Patient Information", "segments": ["MSH", "SFT", "EVN", "PID", "PD1", "ROL", "NK1", "PV1", "PV2", "DB1", "OBX", "AL1", "DG1", "DRG", "PR1", "GT1", "IN1", "IN2", "ACC", "NTE"]},
    "ADT_A09": {"description": "Patient Departing - Tracking", "segments": ["MSH", "SFT", "EVN", "PID", "PD1", "PV1", "PV2", "DB1", "OBX"]},
    "ADT_A10": {"description": "Patient Arriving - Tracking", "segments": ["MSH", "SFT", "EVN", "PID", "PD1", "PV1", "PV2", "DB1", "OBX"]},
    "ADT_A11": {"description": "Cancel Admit/Visit Notification", "segments": ["MSH", "SFT", "EVN", "PID", "PD1", "PV1", "PV2", "DB1", "OBX", "DG1"]},
    "ADT_A12": {"description": "Cancel Transfer", "segments": ["MSH", "SFT", "EVN", "PID", "PD1", "PV1", "PV2", "DB1", "OBX", "DG1"]},
    "ADT_A13": {"description": "Cancel Discharge/End Visit", "segments": ["MSH", "SFT", "EVN", "PID", "PD1", "ROL", "NK1", "PV1", "PV2", "DB1", "OBX", "AL1", "DG1", "DRG", "PR1", "GT1", "IN1", "IN2", "ACC"]},
    "ADT_A17": {"description": "Swap Patients", "segments": ["MSH", "SFT", "EVN", "PID", "PD1", "PV1", "PV2", "DB1", "OBX"]},
    "ADT_A28": {"description": "Add Person Information", "segments": ["MSH", "SFT", "EVN", "PID", "PD1", "ROL", "NK1", "PV1", "PV2", "DB1", "OBX", "AL1", "DG1", "DRG", "IN1", "IN2", "ACC"]},
    "ADT_A31": {"description": "Update Person Information", "segments": ["MSH", "SFT", "EVN", "PID", "PD1", "ROL", "NK1", "PV1", "PV2", "DB1", "OBX", "AL1", "DG1", "DRG", "IN1", "IN2", "ACC"]},
    "ADT_A40": {"description": "Merge Patient - Patient Identifier List", "segments": ["MSH", "SFT", "EVN", "PID", "PD1", "MRG", "PV1"]},
    "BAR_P01": {"description": "Add Patient Accounts", "segments": ["MSH", "SFT", "EVN", "PID", "PD1", "ROL", "PV1", "PV2", "DB1", "OBX", "AL1", "DG1", "DRG", "PR1", "GT1", "NK1", "IN1", "IN2", "ACC", "NTE", "BLG"]},
    "BAR_P02": {"description": "Purge Patient Accounts", "segments": ["MSH", "SFT", "EVN", "PID", "PV1"]},
    "DFT_P03": {"description": "Post Detail Financial Transaction", "segments": ["MSH", "SFT", "EVN", "PID", "PD1", "ROL", "PV1", "PV2", "DB1", "OBX", "DG1", "DRG", "GT1", "IN1", "IN2", "ACC", "FT1", "NTE", "PR1"]},
    "DFT_P11": {"description": "Post Detail Financial Transaction - New", "segments": ["MSH", "SFT", "EVN", "PID", "PD1", "ROL", "PV1", "PV2", "DB1", "OBX", "DG1", "DRG", "GT1", "IN1", "IN2", "ACC", "FT1", "NTE", "PR1"]},
    "MDM_T01": {"description": "Original Document Notification", "segments": ["MSH", "SFT", "EVN", "PID", "PV1", "TXA", "OBX", "NTE"]},
    "MDM_T02": {"description": "Original Document Notification and Content", "segments": ["MSH", "SFT", "EVN", "PID", "PV1", "TXA", "OBX", "NTE"]},
    "MDM_T04": {"description": "Document Status Change Notification and Content", "segments": ["MSH", "SFT", "EVN", "PID", "PV1", "TXA", "OBX", "NTE"]},
    "MFN_M01": {"description": "Master File Notification - Not Otherwise Specified", "segments": ["MSH", "SFT", "MFI", "MFE", "NTE"]},
    "OML_O21": {"description": "Laboratory Order", "segments": ["MSH", "SFT", "NTE", "PID", "PD1", "NK1", "PV1", "PV2", "IN1", "IN2", "GT1", "AL1", "ORC", "TQ1", "TQ2", "OBR", "NTE", "DG1", "OBX", "SPM"]},
    "ORM_O01": {"description": "General Order Message", "segments": ["MSH", "SFT", "NTE", "PID", "PD1", "NK1", "PV1", "PV2", "IN1", "IN2", "GT1", "AL1", "ORC", "OBR", "NTE", "DG1", "OBX", "BLG"]},
    "ORU_R01": {"description": "Unsolicited Observation Message", "segments": ["MSH", "SFT", "NTE", "PID", "PD1", "NK1", "PV1", "PV2", "ORC", "OBR", "NTE", "TQ1", "TQ2", "OBX", "NTE", "SPM"]},
    "OUL_R21": {"description": "Unsolicited Laboratory Observation", "segments": ["MSH", "SFT", "NTE", "PID", "PD1", "PV1", "PV2", "NK1", "ORC", "OBR", "NTE", "TQ1", "TQ2", "OBX", "NTE", "SPM"]},
    "QRY_A19": {"description": "Patient Query", "segments": ["MSH", "SFT", "QRD", "QRF"]},
    "QRY_Q01": {"description": "Query Sent for Immediate Response", "segments": ["MSH", "SFT", "QRD", "QRF"]},
    "QRY_Q02": {"description": "Query Sent for Deferred Response", "segments": ["MSH", "SFT", "QRD", "QRF"]},
    "RAS_O17": {"description": "Pharmacy/Treatment Administration", "segments": ["MSH", "SFT", "NTE", "PID", "PD1", "NK1", "PV1", "PV2", "AL1", "ORC", "TQ1", "TQ2", "RXA", "RXR", "OBX", "NTE"]},
    "RDE_O11": {"description": "Pharmacy/Treatment Encoded Order", "segments": ["MSH", "SFT", "NTE", "PID", "PD1", "NK1", "PV1", "PV2", "IN1", "IN2", "GT1", "AL1", "ORC", "TQ1", "TQ2", "RXE", "RXR", "RXC", "OBX", "NTE"]},
    "RDS_O13": {"description": "Pharmacy/Treatment Dispense", "segments": ["MSH", "SFT", "NTE", "PID", "PD1", "NK1", "PV1", "PV2", "AL1", "ORC", "TQ1", "TQ2", "RXD", "RXR", "RXC", "OBX", "NTE"]},
    "RGV_O15": {"description": "Pharmacy/Treatment Give", "segments": ["MSH", "SFT", "NTE", "PID", "PD1", "NK1", "PV1", "PV2", "AL1", "ORC", "TQ1", "TQ2", "RXG", "RXR", "RXC", "OBX", "NTE"]},
    "RRA_O18": {"description": "Pharmacy/Treatment Administration Acknowledgment", "segments": ["MSH", "SFT", "MSA", "ERR", "NTE", "PID", "ORC", "TQ1", "TQ2", "RXA", "RXR"]},
    "RRD_O14": {"description": "Pharmacy/Treatment Dispense Acknowledgment", "segments": ["MSH", "SFT", "MSA", "ERR", "NTE", "PID", "ORC", "TQ1", "TQ2", "RXD", "RXR", "RXC"]},
    "SIU_S12": {"description": "Notification of New Appointment Booking", "segments": ["MSH", "SCH", "TQ1", "NTE", "PID", "PD1", "PV1", "PV2", "OBX", "DG1", "RGS", "AIS", "AIG", "AIL", "AIP", "NTE"]},
    "SIU_S13": {"description": "Notification of Appointment Rescheduling", "segments": ["MSH", "SCH", "TQ1", "NTE", "PID", "PD1", "PV1", "PV2", "OBX", "DG1", "RGS", "AIS", "AIG", "AIL", "AIP", "NTE"]},
    "SIU_S14": {"description": "Notification of Appointment Modification", "segments": ["MSH", "SCH", "TQ1", "NTE", "PID", "PD1", "PV1", "PV2", "OBX", "DG1", "RGS", "AIS", "AIG", "AIL", "AIP", "NTE"]},
    "SIU_S15": {"description": "Notification of Appointment Cancellation", "segments": ["MSH", "SCH", "TQ1", "NTE", "PID", "PD1", "PV1", "PV2", "OBX", "DG1", "RGS", "AIS", "AIG", "AIL", "AIP", "NTE"]},
}


# =============================================================================
# VERSION-SPECIFIC ADJUSTMENTS
# =============================================================================

def apply_version_adjustments(version, segments_data):
    """Apply version-specific field additions/removals/changes."""
    data = copy.deepcopy(segments_data)

    if version == "2.3":
        # v2.3: PID has 30 fields (remove 31-39)
        if "PID" in data:
            for f in list(data["PID"]["fields"].keys()):
                if int(f) > 30:
                    del data["PID"]["fields"][f]

        # v2.3: MSH has 19 fields (remove 20-21)
        if "MSH" in data:
            for f in ["20", "21"]:
                data["MSH"]["fields"].pop(f, None)

        # v2.3: OBX has 17 fields (remove 18-19)
        if "OBX" in data:
            for f in ["18", "19"]:
                data["OBX"]["fields"].pop(f, None)

        # v2.3: ORC has 19 fields (remove 20-31)
        if "ORC" in data:
            for f in list(data["ORC"]["fields"].keys()):
                if int(f) > 19:
                    del data["ORC"]["fields"][f]

        # v2.3: OBR has 44 fields (remove 45-49)
        if "OBR" in data:
            for f in list(data["OBR"]["fields"].keys()):
                if int(f) > 44:
                    del data["OBR"]["fields"][f]

        # v2.3: RXA has 22 fields
        if "RXA" in data:
            for f in list(data["RXA"]["fields"].keys()):
                if int(f) > 22:
                    del data["RXA"]["fields"][f]

        # v2.3: FT1 has 26 fields
        if "FT1" in data:
            for f in list(data["FT1"]["fields"].keys()):
                if int(f) > 26:
                    del data["FT1"]["fields"][f]

        # v2.3: SCH has 25 fields
        if "SCH" in data:
            for f in list(data["SCH"]["fields"].keys()):
                if int(f) > 25:
                    del data["SCH"]["fields"][f]

        # v2.3: ERR has only field 1
        if "ERR" in data:
            for f in list(data["ERR"]["fields"].keys()):
                if int(f) > 1:
                    del data["ERR"]["fields"][f]

        # v2.3: SPM, TQ1, TQ2, SFT didn't exist yet, but we keep them for forward compat
        # v2.3: CE used instead of CWE everywhere (already the case in our base)

        # v2.3: NK1 has 31 fields
        if "NK1" in data:
            for f in list(data["NK1"]["fields"].keys()):
                if int(f) > 31:
                    del data["NK1"]["fields"][f]

        # v2.3: IN1 has 46 fields
        if "IN1" in data:
            for f in list(data["IN1"]["fields"].keys()):
                if int(f) > 46:
                    del data["IN1"]["fields"][f]

        # v2.3: DG1 has 17 fields
        if "DG1" in data:
            for f in list(data["DG1"]["fields"].keys()):
                if int(f) > 17:
                    del data["DG1"]["fields"][f]

        # v2.3: PR1 has 15 fields
        if "PR1" in data:
            for f in list(data["PR1"]["fields"].keys()):
                if int(f) > 15:
                    del data["PR1"]["fields"][f]

        # v2.3: PV2 has 38 fields
        if "PV2" in data:
            for f in list(data["PV2"]["fields"].keys()):
                if int(f) > 38:
                    del data["PV2"]["fields"][f]

        # v2.3: PD1 has 12 fields
        if "PD1" in data:
            for f in list(data["PD1"]["fields"].keys()):
                if int(f) > 12:
                    del data["PD1"]["fields"][f]

        # v2.3: IN2 has 25 fields
        if "IN2" in data:
            for f in list(data["IN2"]["fields"].keys()):
                if int(f) > 25:
                    del data["IN2"]["fields"][f]

        # v2.3: GT1 has 19 fields
        if "GT1" in data:
            for f in list(data["GT1"]["fields"].keys()):
                if int(f) > 19:
                    del data["GT1"]["fields"][f]

    elif version == "2.4":
        # v2.4: PID has 38 fields (remove 39)
        if "PID" in data:
            data["PID"]["fields"].pop("39", None)

        # v2.4: ORC has 24 fields (remove 25-31)
        if "ORC" in data:
            for f in list(data["ORC"]["fields"].keys()):
                if int(f) > 24:
                    del data["ORC"]["fields"][f]

        # v2.4: OBR has 47 fields (remove 48-49)
        if "OBR" in data:
            for f in ["48", "49"]:
                data["OBR"]["fields"].pop(f, None)

        # v2.4: RXD has 30 fields
        if "RXD" in data:
            for f in list(data["RXD"]["fields"].keys()):
                if int(f) > 30:
                    del data["RXD"]["fields"][f]

        # v2.4: RXE has 38 fields
        if "RXE" in data:
            for f in list(data["RXE"]["fields"].keys()):
                if int(f) > 38:
                    del data["RXE"]["fields"][f]

    elif version == "2.5":
        # v2.5 is our baseline - no adjustments needed
        pass

    elif version == "2.6":
        # v2.6: Some CE fields changed to CWE
        # Apply data type changes for v2.6+
        _upgrade_ce_to_cwe_v26(data)

    elif version == "2.7":
        # v2.7: More CE->CWE changes, PID-40 added
        _upgrade_ce_to_cwe_v26(data)

        if "PID" in data:
            data["PID"]["fields"]["40"] = field(
                "Patient Telecommunication Information", "XTN",
                repeating=True, max_length=250
            )

    elif version == "2.8":
        # v2.8: Complete CE->CWE migration, TS->DTM migration, new fields
        _upgrade_ce_to_cwe_v26(data)
        _upgrade_ts_to_dtm_v28(data)
        _upgrade_remaining_ce_to_cwe_v28(data)

        # PID-40 carried over from v2.7
        if "PID" in data:
            data["PID"]["fields"]["40"] = field(
                "Patient Telecommunication Information", "XTN",
                repeating=True, max_length=250
            )

        # MSH-22..24 added in v2.8
        if "MSH" in data:
            data["MSH"]["fields"]["22"] = field(
                "Sending Responsible Organization", "XON", max_length=567
            )
            data["MSH"]["fields"]["23"] = field(
                "Receiving Responsible Organization", "XON", max_length=567
            )
            data["MSH"]["fields"]["24"] = field(
                "Sending Network Address", "HD", max_length=227
            )
            data["MSH"]["fields"]["25"] = field(
                "Receiving Network Address", "HD", max_length=227
            )

        # OBX-20..25 added in v2.8
        if "OBX" in data:
            data["OBX"]["fields"]["20"] = field(
                "Observation Site", "CWE", repeating=True, max_length=250
            )
            data["OBX"]["fields"]["21"] = field(
                "Observation Instance Identifier", "EI", max_length=427
            )
            data["OBX"]["fields"]["22"] = field(
                "Mood Code", "CNE", max_length=250, table_id="0725"
            )
            data["OBX"]["fields"]["23"] = field(
                "Performing Organization Name", "XON", max_length=567
            )
            data["OBX"]["fields"]["24"] = field(
                "Performing Organization Address", "XAD", max_length=631
            )
            data["OBX"]["fields"]["25"] = field(
                "Performing Organization Medical Director", "XCN", max_length=3002
            )

        # ORC-32..33 added in v2.8
        if "ORC" in data:
            data["ORC"]["fields"]["32"] = field(
                "Advanced Beneficiary Notice Date", "DT", max_length=8
            )
            data["ORC"]["fields"]["33"] = field(
                "Alternate Placer Order Number", "CX", repeating=True, max_length=250
            )

        # OBR-50..54 added in v2.8
        if "OBR" in data:
            data["OBR"]["fields"]["50"] = field(
                "Parent Universal Service Identifier", "CWE", max_length=250
            )
            data["OBR"]["fields"]["51"] = field(
                "Observation Group ID", "EI", max_length=427
            )
            data["OBR"]["fields"]["52"] = field(
                "Parent Observation Group ID", "EI", max_length=427
            )
            data["OBR"]["fields"]["53"] = field(
                "Alternate Placer Order Number", "CX", repeating=True, max_length=250
            )
            data["OBR"]["fields"]["54"] = field(
                "Parent Order", "EIP", max_length=855
            )

    return data


def _upgrade_ce_to_cwe_v26(data):
    """In v2.6+, many CE fields were changed to CWE."""
    # Key fields that changed from CE to CWE in v2.6
    ce_to_cwe_fields = {
        "PID": ["10", "15", "16", "17", "22", "26", "27", "28", "35", "36", "38"],
        "PV1": ["38"],
        "OBR": ["4", "12", "16", "31", "38", "39", "40", "43", "44", "45", "46", "47"],
        "OBX": ["3", "6", "15", "17"],
        "ORC": ["16", "17", "18", "20"],
        "EVN": [],
        "FT1": ["7", "13", "14", "19", "25", "26", "27"],
        "RXA": ["5", "7", "8", "9", "14", "17", "18", "19"],
        "RXD": ["2", "5", "6", "15", "17", "20", "21", "25"],
        "RXE": ["2", "5", "6", "7", "11", "21", "24", "26", "27", "29"],
        "RXG": ["4", "7", "8", "9", "13", "16", "18", "21", "22"],
        "SCH": ["5", "6", "7", "8", "10", "25"],
        "TXA": [],
        "NK1": ["3", "7", "14", "19", "20", "22", "25", "28", "29", "35"],
        "DG1": ["3", "7", "8", "11", "15"],
        "AL1": ["2", "3", "4"],
        "GT1": ["11"],
        "IN1": ["2", "17", "42"],
        "PR1": ["3", "13", "15", "18"],
        "PV2": ["2", "3", "4", "30", "38", "39", "40", "41", "42", "45"],
        "PD1": ["3", "11", "15"],
        "DRG": ["1", "5"],
        "ACC": ["2", "4"],
        "ROL": ["3", "7", "8", "9", "10"],
        "NTE": ["4"],
        "AIS": ["3", "6", "8", "10", "11", "12"],
        "AIG": ["3", "4", "5", "7", "10", "12", "14"],
        "AIL": ["4", "5", "8", "10", "12"],
        "AIP": ["4", "5", "8", "10", "12"],
    }

    for seg_id, field_nums in ce_to_cwe_fields.items():
        if seg_id in data:
            for fn in field_nums:
                if fn in data[seg_id]["fields"]:
                    if data[seg_id]["fields"][fn]["data_type"] == "CE":
                        data[seg_id]["fields"][fn]["data_type"] = "CWE"
                        if "components" in data[seg_id]["fields"][fn]:
                            data[seg_id]["fields"][fn]["components"] = copy.deepcopy(COMPOSITE_COMPONENTS["CWE"])


def _upgrade_remaining_ce_to_cwe_v28(data):
    """In v2.8, virtually all remaining CE fields were changed to CWE."""
    for seg_id, seg in data.items():
        for fn, fld in seg.get("fields", {}).items():
            if fld["data_type"] == "CE":
                fld["data_type"] = "CWE"
                if "components" in fld:
                    fld["components"] = copy.deepcopy(COMPOSITE_COMPONENTS["CWE"])


def _upgrade_ts_to_dtm_v28(data):
    """In v2.8, TS (Timestamp) fields were changed to DTM (Date/Time)."""
    # DTM is a primitive type (no components), while TS was composite with
    # TS.1 = Time and TS.2 = Degree of Precision. v2.8 uses DTM directly.
    for seg_id, seg in data.items():
        for fn, fld in seg.get("fields", {}).items():
            if fld["data_type"] == "TS":
                fld["data_type"] = "DTM"
                # DTM is a primitive type, remove any TS component definitions
                fld.pop("components", None)


# =============================================================================
# VERSION STRINGS
# =============================================================================

VERSION_MAP = {
    "v2_3": "2.3",
    "v2_4": "2.4",
    "v2_5": "2.5",
    "v2_6": "2.6",
    "v2_7": "2.7",
    "v2_8": "2.8",
}


# =============================================================================
# SCHEMA GENERATION
# =============================================================================

def generate_schema(message_key, version_dir, version_str):
    """Generate a complete schema JSON for a message type."""
    msg_def = MESSAGE_DEFINITIONS[message_key]

    # Parse message type and trigger event
    parts = message_key.split("_")
    message_type = parts[0]
    trigger_event = parts[1] if len(parts) > 1 else ""

    # Build segments
    segments = {}
    for seg_id in msg_def["segments"]:
        meta = copy.deepcopy(SEGMENT_META.get(seg_id, {
            "name": seg_id,
            "required": False,
            "repeating": False,
        }))

        # Apply per-message overrides
        if "overrides" in msg_def and seg_id in msg_def["overrides"]:
            meta.update(msg_def["overrides"][seg_id])

        fields_fn = SEGMENT_FIELD_GENERATORS.get(seg_id)
        if fields_fn:
            meta["fields"] = fields_fn()
        else:
            meta["fields"] = {}

        segments[seg_id] = meta

    # Apply version-specific adjustments
    segments = apply_version_adjustments(version_str, segments)

    schema = {
        "message_type": message_type,
        "trigger_event": trigger_event,
        "version": version_str,
        "description": msg_def["description"],
        "segments": segments,
    }

    return schema


def clean_none_values(obj):
    """Remove None/null values from nested dicts for clean JSON output."""
    if isinstance(obj, dict):
        return {k: clean_none_values(v) for k, v in obj.items() if v is not None}
    elif isinstance(obj, list):
        return [clean_none_values(v) for v in obj]
    return obj


def main():
    base_dir = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                            "crates", "vyna-hl7-validator", "schemas")

    total_files = 0
    total_fields = 0
    total_components = 0

    for version_dir, version_str in VERSION_MAP.items():
        schema_dir = os.path.join(base_dir, version_dir)
        os.makedirs(schema_dir, exist_ok=True)

        for message_key in sorted(MESSAGE_DEFINITIONS.keys()):
            schema = generate_schema(message_key, version_dir, version_str)
            schema = clean_none_values(schema)

            filename = f"{message_key}.json"
            filepath = os.path.join(schema_dir, filename)

            with open(filepath, 'w') as f:
                json.dump(schema, f, indent=2)
                f.write('\n')

            # Count stats
            total_files += 1
            for seg in schema["segments"].values():
                for fld in seg.get("fields", {}).values():
                    total_fields += 1
                    if "components" in fld:
                        total_components += len(fld["components"])

    print(f"Generated {total_files} schema files")
    print(f"Total field definitions: {total_fields}")
    print(f"Total component definitions: {total_components}")

    # Print per-version stats
    for version_dir, version_str in VERSION_MAP.items():
        schema_dir = os.path.join(base_dir, version_dir)
        files = [f for f in os.listdir(schema_dir) if f.endswith('.json')]
        print(f"  {version_dir}: {len(files)} files")


if __name__ == "__main__":
    main()
