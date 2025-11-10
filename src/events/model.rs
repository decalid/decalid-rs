use std::collections::BTreeMap;
use std::str::FromStr;

use anyhow::{anyhow, Result};

use chrono::DateTime;
use chrono::TimeZone;
use chrono::Utc;
use chrono_tz::Tz as ChronoTz;
use ical::parser::Component as _;

use crate::chrono_utils::TimeDeltaToString as _;

type EventPropertyParams = BTreeMap<String, Vec<String>>;
type EventPropertyValue = Option<String>;
type EventProperty = (EventPropertyParams, EventPropertyValue);
pub(crate) type EventPropertyMap = BTreeMap<String, Vec<EventProperty>>;

/// The internal structure of a Parsed Event
///
/// Usually follows RFC 5545 but does not require compliance with all the
/// alternatives. For example, we will always specify a dtend and never a
/// duration unless we are given an event with one, but we'll canonicalize it
/// into dtend.
#[derive(Clone, Debug)]
pub struct ParsedEvent {
    /// Alternate text representation, if available. If defined, it must be a
    /// URI pointing to an alternate representation for a textual property
    /// value.
    pub dtstamp: String,
    pub uid: Option<String>,
    pub dtstart: Option<DateTime<rrule::Tz>>,
    pub class: Option<String>,
    pub created: Option<String>,
    pub description: Option<String>,
    pub geo: Option<String>,
    pub last_mod: Option<String>,
    pub location: Option<String>,
    pub organizer: Option<String>,
    pub priority: Option<String>,
    pub seq: Option<String>,
    pub status: Option<String>,
    pub summary: Option<String>,
    pub transp: Option<String>,
    pub url: Option<String>,
    pub recurid: Option<String>,
    pub rrule: Option<String>,
    pub dtend: Option<DateTime<rrule::Tz>>,
    pub _all_day: bool,
    pub _last_repeat: Option<DateTime<rrule::Tz>>,
    pub duration: Option<String>,

    pub attach: Vec<EventProperty>,
    pub attendee: Vec<EventProperty>,
    pub categories: Vec<EventProperty>,
    pub comment: Vec<EventProperty>,
    pub contact: Vec<EventProperty>,
    pub exdate: Vec<EventProperty>,
    pub rstatus: Vec<EventProperty>,
    pub related: Vec<EventProperty>,
    pub resources: Vec<EventProperty>,
    pub rdate: Vec<DateTime<rrule::Tz>>,
    pub x_prop: Vec<EventProperty>,
    pub iana_prop: Vec<EventProperty>,
    pub valarms: Vec<ParsedValarm>,

    pub inner: ical::parser::ical::component::IcalEvent,
}

#[derive(Clone, Debug, Default)]
pub struct ParsedValarm {
    pub action: String,
    pub trigger: String,
    pub description: Option<String>,
    pub duration: Option<String>,
}

#[allow(unused)]
#[derive(Clone)]
pub struct DecalidEvent {
    pub inner: ParsedEvent,
    pub summary: Option<String>,
    pub first_start_time: DateTime<Utc>,
    pub last_end_time: DateTime<Utc>,
    pub is_all_day: bool,
    pub recurrence_set: rrule::RRuleSet,
}

fn slice2btreemap<KA: Ord, KB: Ord, B: Clone>(
    vec: &[(KA, B)],
    f_keys: &dyn Fn(&KA) -> KB,
) -> BTreeMap<KB, B> {
    let mut map = BTreeMap::new();
    for (key, value) in vec {
        map.insert(f_keys(key), value.clone());
    }
    map
}

pub(crate) fn props2btree(props: &[ical::property::Property]) -> EventPropertyMap {
    let mut known_props = BTreeMap::new();

    for prop in props.iter() {
        let prop_name = prop.name.to_string();
        if !known_props.contains_key(&prop_name) {
            known_props.insert(prop_name.clone(), Vec::new());
        }
        let existing = known_props.get_mut(&prop_name).unwrap();
        existing.push((
            slice2btreemap(prop.params.as_deref().unwrap_or_default(), &|str| {
                str.to_ascii_uppercase()
            }),
            prop.value.clone(),
        ));
    }
    known_props
}

fn is_datetime(str: &str) -> bool {
    str.contains('T')
}

fn ical_datetime_or_date_to_rust_datetime(
    prop: Option<&EventProperty>,
    default_timezone: Option<ChronoTz>,
) -> Result<Option<DateTime<rrule::Tz>>> {
    use chrono::NaiveDateTime;

    if let Some((params, input)) = prop {
        if let Some(input_ref) = input.as_deref() {
            // First check if we have a TZID parameter
            let tzid = params.get("TZID").and_then(|v| v.first()).cloned();
            let has_z_suffix = input_ref.ends_with('Z');

            // Parse the datetime string based on whether it contains 'T' (datetime) or not (date)
            let naive_dt = if is_datetime(input_ref) {
                // Parse as datetime (UTC values end with 'Z'; local times omit it per RFC 5545 §3.3.5)
                if has_z_suffix {
                    NaiveDateTime::parse_from_str(input_ref, "%Y%m%dT%H%M%SZ")?
                } else {
                    NaiveDateTime::parse_from_str(input_ref, "%Y%m%dT%H%M%S")?
                }
            } else {
                // Parse as date with time set to midnight
                chrono::NaiveDate::parse_from_str(input_ref, "%Y%m%d")?
                    .and_hms_opt(0, 0, 0)
                    .ok_or_else(|| anyhow::anyhow!("Invalid time"))?
            };

            if has_z_suffix && tzid.is_some() {
                return Err(anyhow!(
                    "DATE-TIME value '{input_ref}' includes both a TZID parameter and a 'Z' suffix"
                ));
            }

            if has_z_suffix {
                return Ok(Some(rrule::Tz::UTC.from_utc_datetime(&naive_dt)));
            }

            // If we have a TZID, use it, otherwise assume UTC.
            // RFC 5545 §3.2.19 requires DATE-TIME values with TZID parameters to be floating (no trailing 'Z').
            if let Some(tz_name) = tzid {
                let tz: ChronoTz = tz_name
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid timezone: {tz_name}"))?;
                Ok(Some(
                    tz.from_local_datetime(&naive_dt)
                        .earliest()
                        .ok_or_else(|| anyhow::anyhow!("Invalid timezone conversion"))?
                        .with_timezone(&rrule::Tz::UTC),
                ))
            } else if is_datetime(input_ref) {
                if let Some(tz) = default_timezone {
                    Ok(Some(
                        tz.from_local_datetime(&naive_dt)
                            .earliest()
                            .ok_or_else(|| anyhow::anyhow!("Invalid timezone conversion"))?
                            .with_timezone(&rrule::Tz::UTC),
                    ))
                } else {
                    Ok(Some(rrule::Tz::UTC.from_utc_datetime(&naive_dt)))
                }
            } else {
                Ok(Some(rrule::Tz::UTC.from_utc_datetime(&naive_dt)))
            }
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

fn _all_props_ok(_params: &EventPropertyParams) -> bool {
    true
}

#[allow(unused)]
fn debug_print_props(props: &EventPropertyMap) {
    println!("[PROPS BEGIN]");
    for (prop_name, prop_list) in props.iter() {
        println!("Property {prop_name}: {prop_list:?}");
    }
    println!("[PROPS END]");
}

fn parse_single_prop<T>(
    props: &EventPropertyMap,
    prop_name: &str,
    prop_filter: Option<&dyn Fn(&EventPropertyParams) -> bool>,
    parser: &dyn Fn(Option<&EventProperty>) -> Result<T>,
) -> Result<T> {
    let prop_filter = prop_filter.unwrap_or(&_all_props_ok);
    let prop = props
        .get(prop_name)
        .ok_or_else(|| anyhow::anyhow!("Property not found: {prop_name}"))?
        .iter()
        .find(|f| prop_filter(&f.0));
    parser(prop)
}

fn parse_single_prop_opt<T>(
    props: &EventPropertyMap,
    prop_name: &str,
    prop_filter: Option<&dyn Fn(&EventPropertyParams) -> bool>,
    parser: &dyn Fn(Option<&EventProperty>) -> Result<Option<T>>,
) -> Result<Option<T>> {
    let prop_filter = prop_filter.unwrap_or(&_all_props_ok);
    if let Some(prop) = props.get(prop_name) {
        parser(prop.iter().find(|f| prop_filter(&f.0)))
    } else {
        Ok(None)
    }
}

fn parse_multiple_props<T>(
    props: &EventPropertyMap,
    prop_name: &str,
    prop_filter: Option<&dyn Fn(&EventPropertyParams) -> bool>,
    parser: &dyn Fn(&EventProperty) -> Result<T>,
) -> Result<Vec<T>> {
    let prop_filter = prop_filter.unwrap_or(&_all_props_ok);
    let props = props.get(prop_name).map_or(Vec::new(), |props| {
        props.iter().filter(|prop| prop_filter(&prop.0)).collect()
    });

    let mut parsed_props = Vec::new();
    for prop in props {
        parsed_props.push(parser(prop)?);
    }
    Ok(parsed_props)
}

fn get_cloned_prop_value(s: Option<&EventProperty>) -> Result<Option<String>> {
    Ok(s.and_then(|inner| inner.1.clone()))
}

impl ParsedEvent {
    pub fn new(event: ical::parser::ical::component::IcalEvent) -> Result<ParsedEvent> {
        Self::with_default_timezone(event, None)
    }

    pub fn with_default_timezone(
        event: ical::parser::ical::component::IcalEvent,
        default_timezone: Option<ChronoTz>,
    ) -> Result<ParsedEvent> {
        let known_props = props2btree(&event.properties);
        let _all_day = parse_single_prop_opt(&known_props, "DTSTART", None, &|v| {
            Ok(v.and_then(|t| t.1.as_ref().filter(|s| is_datetime(s)).map(|_| true)))
        })?
        .is_some();

        let rrule = parse_single_prop_opt(&known_props, "RRULE", None, &get_cloned_prop_value)?;
        let dtstart = parse_single_prop_opt(&known_props, "DTSTART", None, &|prop| {
            ical_datetime_or_date_to_rust_datetime(prop, default_timezone)
        })?;
        let mut dtend = parse_single_prop_opt(&known_props, "DTEND", None, &|prop| {
            ical_datetime_or_date_to_rust_datetime(prop, default_timezone)
        })?;

        let mut duration =
            parse_single_prop_opt(&known_props, "DURATION", None, &get_cloned_prop_value)?;

        // We must ensure there is both dtend and duration.
        // First, fill dtend in case it does not exist, but duration does:
        if dtend.is_none() {
            if let (Some(dtstart_value), Some(duration_value)) =
                (dtstart.as_ref(), duration.as_ref())
            {
                if let Ok(duration_parsed) = iso8601_duration::Duration::parse(duration_value) {
                    let chrono_duration = duration_parsed.to_chrono_at_datetime(*dtstart_value);
                    dtend = Some(*dtstart_value + chrono_duration);
                }
            }
        } else if duration.is_none() {
            if let (Some(dtend_value), Some(dtstart_value)) = (dtend.as_ref(), dtstart.as_ref()) {
                let duration_ = *dtend_value - *dtstart_value;
                duration = Some(duration_.to_iso8601_string())
            }
        }

        let rdate = parse_multiple_props(&known_props, "RDATE", None, &|v| {
            ical_datetime_or_date_to_rust_datetime(Some(v), default_timezone)?
                .ok_or_else(|| anyhow!("RDATE is missing a value"))
        })?;

        let _last_repeat = if rrule.is_none() {
            dtend
        } else {
            // Calculate last repeat
            let rrule =
                rrule::RRule::from_str(rrule.as_ref().unwrap())?.validate(dtstart.unwrap())?;
            let dates = rrule::RRuleSet::new(dtstart.unwrap())
                .rrule(rrule)
                .set_rdates(rdate.clone())
                .all(15000)
                .dates;
            dates.last().cloned()
        };

        let valarms = parse_multiple_props(&known_props, "VALARM", None, &|v| {
            let (trigger, action) = v;
            let action = action.iter().next().cloned().expect("action must exist");
            let trigger_str = trigger
                .get("TRIGGER")
                .and_then(|v| v.first())
                .cloned()
                .unwrap_or_default();
            let description = Some(action.clone());
            let duration = trigger.get("DURATION").and_then(|v| v.first()).cloned();
            Ok(ParsedValarm {
                action,
                trigger: trigger_str,
                description,
                duration,
            })
        })?;

        Ok(ParsedEvent {
            _all_day,
            _last_repeat,

            dtstamp: parse_single_prop(&known_props, "DTSTAMP", None, &|v| {
                Ok(v.and_then(|t| t.1.clone()))
            })?
            .expect("dtstamp property must exist"),
            uid: parse_single_prop(&known_props, "UID", None, &|v| {
                Ok(v.and_then(|t| t.1.clone()))
            })?,
            dtstart,
            class: parse_single_prop_opt(&known_props, "CLASS", None, &get_cloned_prop_value)?,
            created: parse_single_prop_opt(&known_props, "CREATED", None, &get_cloned_prop_value)?,
            description: parse_single_prop_opt(
                &known_props,
                "DESCRIPTION",
                None,
                &get_cloned_prop_value,
            )?,
            geo: parse_single_prop_opt(&known_props, "GEO", None, &get_cloned_prop_value)?,
            last_mod: parse_single_prop_opt(
                &known_props,
                "LAST_MOD",
                None,
                &get_cloned_prop_value,
            )?,
            location: parse_single_prop_opt(
                &known_props,
                "LOCATION",
                None,
                &get_cloned_prop_value,
            )?,
            organizer: parse_single_prop_opt(
                &known_props,
                "ORGANIZER",
                None,
                &get_cloned_prop_value,
            )?,
            priority: parse_single_prop_opt(
                &known_props,
                "PRIORITY",
                None,
                &get_cloned_prop_value,
            )?,
            seq: parse_single_prop_opt(&known_props, "SEQ", None, &get_cloned_prop_value)?,
            status: parse_single_prop_opt(&known_props, "STATUS", None, &get_cloned_prop_value)?,
            summary: parse_single_prop_opt(&known_props, "SUMMARY", None, &get_cloned_prop_value)?,
            transp: parse_single_prop_opt(&known_props, "TRANSP", None, &get_cloned_prop_value)?,
            url: parse_single_prop_opt(&known_props, "URL", None, &get_cloned_prop_value)?,
            recurid: parse_single_prop_opt(&known_props, "RECURID", None, &get_cloned_prop_value)?,
            rrule,
            dtend,
            duration,

            attach: parse_multiple_props(&known_props, "ATTACH", None, &|v| Ok(v.clone()))?,
            attendee: parse_multiple_props(&known_props, "ATTENDEE", None, &|v| Ok(v.clone()))?,
            categories: parse_multiple_props(&known_props, "CATEGORIES", None, &|v| Ok(v.clone()))?,
            comment: parse_multiple_props(&known_props, "COMMENT", None, &|v| Ok(v.clone()))?,
            contact: parse_multiple_props(&known_props, "CONTACT", None, &|v| Ok(v.clone()))?,
            exdate: parse_multiple_props(&known_props, "EXDATE", None, &|v| Ok(v.clone()))?,
            rstatus: parse_multiple_props(&known_props, "RSTATUS", None, &|v| Ok(v.clone()))?,
            related: parse_multiple_props(&known_props, "RELATED", None, &|v| Ok(v.clone()))?,
            resources: parse_multiple_props(&known_props, "RESOURCES", None, &|v| Ok(v.clone()))?,
            rdate,
            x_prop: parse_multiple_props(&known_props, "X_PROP", None, &|v| Ok(v.clone()))?,
            iana_prop: parse_multiple_props(&known_props, "IANA_PROP", None, &|v| Ok(v.clone()))?,
            valarms,

            inner: event,
        })
    }

    pub fn serialize(&self) -> String {
        let mut output = String::new();

        for property in &self.inner.properties {
            let value = property.value.as_deref().unwrap_or("");
            let params = property
                .params
                .as_ref()
                .map(|p| p.iter())
                .unwrap_or_else(|| [].iter())
                .map(|(k, v)| format!("{}={}", k, v.join(",")))
                .collect::<Vec<_>>()
                .join(";");

            if params.is_empty() {
                output.push_str(&format!("{}:{}\r\n", property.name, value));
            } else {
                output.push_str(&format!("{};{}:{}\r\n", property.name, params, value));
            }
        }
        output
    }
}

impl From<&crate::db::models::EventVersion> for DecalidEvent {
    fn from(value: &crate::db::models::EventVersion) -> Self {
        let event = {
            let mut event = ical::parser::ical::component::IcalEvent::new();
            {
                let mut buffer = value.raw_data.clone();
                buffer.push_str("END:VEVENT\r\n"); // This is due to the implementation of the ical:: parser crate
                let buffer = buffer.as_bytes();
                let parser = std::cell::RefCell::new(ical::PropertyParser::from_reader(
                    std::io::BufReader::new(buffer),
                ));
                if let Err(x) = event.parse(&parser) {
                    panic!("Got a ParserError: {x:#?}")
                }
            }
            ParsedEvent::new(event).unwrap()
        };

        let dtstart = event.dtstart.unwrap();
        let mut recurrence_set = rrule::RRuleSet::new(dtstart);
        let rrule = event.rrule.as_deref().and_then(|rrule_str| {
            rrule::RRule::from_str(rrule_str)
                .ok()
                .and_then(|rrule| rrule::RRule::validate(rrule, dtstart).ok())
        });
        if let Some(rrule) = rrule {
            recurrence_set = recurrence_set.rrule(rrule);
        }
        for exdate in &event.exdate {
            let exdate = ical_datetime_or_date_to_rust_datetime(Some(exdate), None)
                .ok()
                .flatten()
                .unwrap();
            recurrence_set = recurrence_set.exdate(exdate);
        }

        DecalidEvent {
            summary: event.summary.clone(),
            first_start_time: dtstart.to_utc(),
            last_end_time: value
                .last_repeat
                .expect("Last repeat must exist in the database"),
            is_all_day: value.is_all_day, // TODO: Implement all-day detection
            recurrence_set,
            inner: event,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use chrono::{NaiveDateTime, TimeZone};
    use chrono_tz::Tz as ChronoTz;

    #[test]
    fn test_basic_rrule_parsing() -> Result<()> {
        let mut event = ical::parser::ical::component::IcalEvent::new();
        event.properties = vec![
            ical::property::Property {
                name: "DTSTAMP".to_string(),
                params: None,
                value: Some("20240101T100000".to_string()),
            },
            ical::property::Property {
                name: "UID".to_string(),
                params: None,
                value: Some("1234".to_string()),
            },
            ical::property::Property {
                name: "DTSTART".to_string(),
                params: None,
                value: Some("20240101T100000".to_string()),
            },
            ical::property::Property {
                name: "RRULE".to_string(),
                params: None,
                value: Some("FREQ=DAILY;COUNT=3".to_string()),
            },
        ];

        let parsed = ParsedEvent::new(event).unwrap();
        let start_time = rrule::Tz::UTC
            .with_ymd_and_hms(2024, 1, 1, 10, 0, 0)
            .unwrap();

        let mut recurrence_set = rrule::RRuleSet::new(start_time);
        if let Some(rrule) = parsed.rrule {
            let rrule = rrule::RRule::from_str(&rrule)?.validate(start_time)?;
            recurrence_set = recurrence_set.rrule(rrule);
        }

        let occurrences: Vec<_> = recurrence_set.all(15000).dates;

        println!("GOT OCCURRENCES: {:?}", occurrences);

        assert_eq!(occurrences.len(), 3);
        assert_eq!(
            occurrences[0],
            Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap()
        );
        assert_eq!(
            occurrences[1],
            Utc.with_ymd_and_hms(2024, 1, 2, 10, 0, 0).unwrap()
        );
        assert_eq!(
            occurrences[2],
            Utc.with_ymd_and_hms(2024, 1, 3, 10, 0, 0).unwrap()
        );
        Ok(())
    }

    #[test]
    fn test_rrule_with_exdate() -> Result<()> {
        let mut event = ical::parser::ical::component::IcalEvent::new();
        event.properties = vec![
            ical::property::Property {
                name: "DTSTAMP".to_string(),
                params: None,
                value: Some("20240101T100000".to_string()),
            },
            ical::property::Property {
                name: "UID".to_string(),
                params: None,
                value: Some("1234".to_string()),
            },
            ical::property::Property {
                name: "DTSTART".to_string(),
                params: None,
                value: Some("20240101T100000".to_string()),
            },
            ical::property::Property {
                name: "RRULE".to_string(),
                params: None,
                value: Some("FREQ=DAILY;COUNT=4".to_string()),
            },
            ical::property::Property {
                name: "EXDATE".to_string(),
                params: None,
                value: Some("20240102T100000".to_string()),
            },
        ];

        let parsed = ParsedEvent::new(event).unwrap();
        let start_time = rrule::Tz::UTC
            .with_ymd_and_hms(2024, 1, 1, 10, 0, 0)
            .unwrap();

        let mut recurrence_set = rrule::RRuleSet::new(start_time);
        if let Some(rrule) = parsed.rrule {
            let rrule = rrule::RRule::from_str(&rrule)?.validate(start_time)?;
            recurrence_set = recurrence_set.rrule(rrule);
        }
        for exdate in &parsed.exdate {
            let (_, value) = exdate;
            if let Some(date_str) = value {
                if let Ok(dt) = NaiveDateTime::parse_from_str(&date_str, "%Y%m%dT%H%M%S") {
                    let utc_dt = rrule::Tz::UTC.from_utc_datetime(&dt);
                    recurrence_set = recurrence_set.exdate(utc_dt);
                }
            }
        }

        let occurrences: Vec<_> = recurrence_set.into_iter().take(3).collect();

        assert_eq!(occurrences.len(), 3);
        assert_eq!(
            occurrences[0],
            Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap()
        );
        // January 2nd is excluded
        assert_eq!(
            occurrences[1],
            Utc.with_ymd_and_hms(2024, 1, 3, 10, 0, 0).unwrap()
        );
        assert_eq!(
            occurrences[2],
            Utc.with_ymd_and_hms(2024, 1, 4, 10, 0, 0).unwrap()
        );
        Ok(())
    }

    #[test]
    fn test_dtstart_with_tzid_without_trailing_z() -> Result<()> {
        let mut event = ical::parser::ical::component::IcalEvent::new();
        event.properties = vec![
            ical::property::Property {
                name: "DTSTAMP".to_string(),
                params: None,
                value: Some("20240101T050000Z".to_string()),
            },
            ical::property::Property {
                name: "UID".to_string(),
                params: None,
                value: Some("abc".to_string()),
            },
            ical::property::Property {
                name: "DTSTART".to_string(),
                params: Some(vec![(
                    "TZID".to_string(),
                    vec!["America/New_York".to_string()],
                )]),
                value: Some("20240101T050000".to_string()),
            },
        ];

        let parsed = ParsedEvent::new(event)?;
        let dtstart = parsed.dtstart.expect("dtstart should parse");

        assert_eq!(
            dtstart,
            rrule::Tz::UTC
                .with_ymd_and_hms(2024, 1, 1, 10, 0, 0)
                .single()
                .expect("valid datetime"),
        );

        Ok(())
    }

    #[test]
    fn test_dtstart_with_trailing_z_and_tzid_errors() {
        let mut event = ical::parser::ical::component::IcalEvent::new();
        event.properties = vec![
            ical::property::Property {
                name: "DTSTAMP".to_string(),
                params: None,
                value: Some("20240101T050000Z".to_string()),
            },
            ical::property::Property {
                name: "UID".to_string(),
                params: None,
                value: Some("abc".to_string()),
            },
            ical::property::Property {
                name: "DTSTART".to_string(),
                params: Some(vec![(
                    "TZID".to_string(),
                    vec!["America/New_York".to_string()],
                )]),
                value: Some("20240101T050000Z".to_string()),
            },
        ];

        let result = ParsedEvent::with_default_timezone(event, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_floating_dtstart_uses_default_timezone() -> Result<()> {
        let mut event = ical::parser::ical::component::IcalEvent::new();
        event.properties = vec![
            ical::property::Property {
                name: "DTSTAMP".to_string(),
                params: None,
                value: Some("20240101T050000Z".to_string()),
            },
            ical::property::Property {
                name: "UID".to_string(),
                params: None,
                value: Some("floating".to_string()),
            },
            ical::property::Property {
                name: "DTSTART".to_string(),
                params: None,
                value: Some("20240101T090000".to_string()),
            },
        ];

        let default_tz: ChronoTz = "America/New_York".parse().unwrap();
        let parsed = ParsedEvent::with_default_timezone(event, Some(default_tz))?;
        let dtstart = parsed.dtstart.expect("dtstart should parse");

        assert_eq!(
            dtstart,
            rrule::Tz::UTC
                .with_ymd_and_hms(2024, 1, 1, 14, 0, 0)
                .single()
                .expect("valid datetime"),
        );

        Ok(())
    }

    #[test]
    fn test_floating_dtstart_without_default_timezone_falls_back_to_utc() -> Result<()> {
        let mut event = ical::parser::ical::component::IcalEvent::new();
        event.properties = vec![
            ical::property::Property {
                name: "DTSTAMP".to_string(),
                params: None,
                value: Some("20240101T050000Z".to_string()),
            },
            ical::property::Property {
                name: "UID".to_string(),
                params: None,
                value: Some("floating".to_string()),
            },
            ical::property::Property {
                name: "DTSTART".to_string(),
                params: None,
                value: Some("20240101T090000".to_string()),
            },
        ];

        let parsed = ParsedEvent::new(event)?;
        let dtstart = parsed.dtstart.expect("dtstart should parse");

        assert_eq!(
            dtstart,
            rrule::Tz::UTC
                .with_ymd_and_hms(2024, 1, 1, 9, 0, 0)
                .single()
                .expect("valid datetime"),
        );

        Ok(())
    }
}
