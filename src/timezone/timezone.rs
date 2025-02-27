use anyhow::{anyhow, Result};
use chrono::{DateTime, TimeZone, Utc};
use ical::parser::ical::component::{IcalTimeZone, IcalTimeZoneTransition, IcalTimeZoneTransitionType};

use crate::db::models::{Timezone, TimezoneRule};
use crate::events::model::{EventPropertyMap, props2btree};

/// RFC 7809 timezone reference prefix
pub const TIMEZONE_REFERENCE_PREFIX: &str = "TZID:";

/// Represents a parsed VTIMEZONE component
#[derive(Debug, Clone)]
pub struct ParsedTimezone {
    pub tzid: String,
    pub standard_rules: Vec<TimezoneRuleComponent>,
    pub daylight_rules: Vec<TimezoneRuleComponent>,
    pub raw_data: String,
}

/// Represents a timezone rule (STANDARD or DAYLIGHT component)
#[derive(Debug, Clone)]
pub struct TimezoneRuleComponent {
    pub dtstart: DateTime<Utc>,
    pub tzoffsetfrom: String,
    pub tzoffsetto: String,
    pub rrule: Option<String>,
}

impl ParsedTimezone {
    /// Create a new ParsedTimezone from an IcalTimeZone component
    pub fn new(timezone: IcalTimeZone) -> Result<Self> {
        let props = props2btree(&timezone.properties);
        
        // Get TZID (required)
        let tzid = get_tzid(&props)?;
        
        // Parse STANDARD and DAYLIGHT transitions
        let mut standard_rules = Vec::new();
        let mut daylight_rules = Vec::new();
        
        for transition in &timezone.transitions {
            match transition.transition {
                IcalTimeZoneTransitionType::STANDARD => {
                    let rule = parse_timezone_transition(transition)?;
                    standard_rules.push(rule);
                },
                IcalTimeZoneTransitionType::DAYLIGHT => {
                    let rule = parse_timezone_transition(transition)?;
                    daylight_rules.push(rule);
                }
            }
        }
        
        // Serialize the raw data
        let raw_data = serialize_timezone(&timezone)?;
        
        Ok(ParsedTimezone {
            tzid,
            standard_rules,
            daylight_rules,
            raw_data,
        })
    }
    
    /// Convert to database model
    pub fn to_db_model(&self) -> Timezone {
        Timezone {
            id: 0, // Will be set by the database
            tzid: self.tzid.clone(),
            raw_data: self.raw_data.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
    
    /// Convert timezone rules to database models
    pub fn rules_to_db_models(&self, timezone_id: i64) -> Vec<TimezoneRule> {
        let mut rules = Vec::new();
        
        // Add STANDARD rules
        for rule in &self.standard_rules {
            rules.push(TimezoneRule {
                id: 0, // Will be set by the database
                timezone_id,
                rule_type: "STANDARD".to_string(),
                dtstart: rule.dtstart,
                tzoffsetfrom: rule.tzoffsetfrom.clone(),
                tzoffsetto: rule.tzoffsetto.clone(),
                rrule: rule.rrule.clone(),
                created_at: Utc::now(),
            });
        }
        
        // Add DAYLIGHT rules
        for rule in &self.daylight_rules {
            rules.push(TimezoneRule {
                id: 0, // Will be set by the database
                timezone_id,
                rule_type: "DAYLIGHT".to_string(),
                dtstart: rule.dtstart,
                tzoffsetfrom: rule.tzoffsetfrom.clone(),
                tzoffsetto: rule.tzoffsetto.clone(),
                rrule: rule.rrule.clone(),
                created_at: Utc::now(),
            });
        }
        
        rules
    }
}

/// Parse a timezone transition component (STANDARD or DAYLIGHT)
fn parse_timezone_transition(transition: &IcalTimeZoneTransition) -> Result<TimezoneRuleComponent> {
    let props = props2btree(&transition.properties);
    
    // Get required properties
    let dtstart = get_datetime_property(&props, "DTSTART")?;
    let tzoffsetfrom = get_string_property(&props, "TZOFFSETFROM")?;
    let tzoffsetto = get_string_property(&props, "TZOFFSETTO")?;
    
    // Get optional RRULE
    let rrule = get_optional_string_property(&props, "RRULE");
    
    Ok(TimezoneRuleComponent {
        dtstart,
        tzoffsetfrom,
        tzoffsetto,
        rrule,
    })
}

/// Get TZID from properties
fn get_tzid(props: &EventPropertyMap) -> Result<String> {
    get_string_property(props, "TZID")
}

/// Get a required string property
fn get_string_property(props: &EventPropertyMap, name: &str) -> Result<String> {
    if let Some(prop_vec) = props.get(name) {
        if let Some(prop) = prop_vec.first() {
            if let Some(value) = &prop.1 {
                return Ok(value.clone());
            }
        }
    }
    
    Err(anyhow!("Missing required property: {}", name))
}

/// Get an optional string property
fn get_optional_string_property(props: &EventPropertyMap, name: &str) -> Option<String> {
    props.get(name)
        .and_then(|prop_vec| prop_vec.first())
        .and_then(|prop| prop.1.clone())
}

/// Get a datetime property
fn get_datetime_property(props: &EventPropertyMap, name: &str) -> Result<DateTime<Utc>> {
    let datetime_str = get_string_property(props, name)?;
    
    // Parse datetime in the format: 20160306T020000Z
    let format = if datetime_str.ends_with('Z') {
        "%Y%m%dT%H%M%SZ"
    } else {
        "%Y%m%dT%H%M%S"
    };
    
    chrono::NaiveDateTime::parse_from_str(&datetime_str, format)
        .map_err(|e| anyhow!("Failed to parse datetime {}: {}", datetime_str, e))
        .map(|dt| Utc.from_utc_datetime(&dt))
}

/// Serialize a timezone component to string
fn serialize_timezone(timezone: &IcalTimeZone) -> Result<String> {
    let mut buffer = String::new();
    
    buffer.push_str("BEGIN:VTIMEZONE\r\n");
    
    // Add properties
    for prop in &timezone.properties {
        buffer.push_str(&format!("{}:{}\r\n", prop.name, prop.value.as_deref().unwrap_or("")));
    }
    
    // Add transition components (STANDARD and DAYLIGHT)
    for transition in &timezone.transitions {
        match transition.transition {
            IcalTimeZoneTransitionType::STANDARD => {
                buffer.push_str("BEGIN:STANDARD\r\n");
            },
            IcalTimeZoneTransitionType::DAYLIGHT => {
                buffer.push_str("BEGIN:DAYLIGHT\r\n");
            }
        }
        
        // Add transition properties
        for prop in &transition.properties {
            buffer.push_str(&format!("{}:{}\r\n", prop.name, prop.value.as_deref().unwrap_or("")));
        }
        
        match transition.transition {
            IcalTimeZoneTransitionType::STANDARD => {
                buffer.push_str("END:STANDARD\r\n");
            },
            IcalTimeZoneTransitionType::DAYLIGHT => {
                buffer.push_str("END:DAYLIGHT\r\n");
            }
        }
    }
    
    buffer.push_str("END:VTIMEZONE\r\n");
    
    Ok(buffer)
}

/// Check if a TZID is a reference (RFC 7809)
pub fn is_timezone_reference(tzid: &str) -> bool {
    tzid.starts_with(TIMEZONE_REFERENCE_PREFIX)
}

/// Extract the actual timezone ID from a reference
pub fn extract_timezone_id_from_reference(tzid: &str) -> Option<String> {
    if is_timezone_reference(tzid) {
        Some(tzid[TIMEZONE_REFERENCE_PREFIX.len()..].to_string())
    } else {
        None
    }
}

/// Resolve a timezone reference to an actual timezone ID
pub fn resolve_timezone_reference(tzid: &str) -> String {
    if let Some(actual_tzid) = extract_timezone_id_from_reference(tzid) {
        actual_tzid
    } else {
        tzid.to_string()
    }
}
