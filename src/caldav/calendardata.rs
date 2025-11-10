#![allow(dead_code)]

use yaserde::{YaDeserialize, YaSerialize};

use super::propfind::Prop;

/// calendar-query root
#[derive(YaSerialize, YaDeserialize, Debug, Default)]
#[yaserde(
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"},
    prefix = "cal",
    rename = "calendar-query",
)]
pub(super) struct CalendarQuery {
    #[yaserde(prefix = "d", rename = "prop")]
    pub prop: Vec<Prop>,
    #[yaserde(prefix = "cal", rename = "filter")]
    pub filter: Option<CalendarQueryFilter>,
}

/// sync-collection root
#[derive(YaSerialize, YaDeserialize, Debug, Default)]
#[yaserde(
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"},
    prefix = "d",
    rename = "sync-collection",
)]
pub(super) struct SyncCollection {
    #[yaserde(prefix = "d", rename = "prop")]
    pub prop: Vec<Prop>,

    #[yaserde(prefix = "d", rename = "sync-token")]
    pub sync_token: Option<super::propfind::Content>,

    #[yaserde(prefix = "d", rename = "sync-level")]
    pub sync_level: Option<i64>,
}

#[derive(YaSerialize, YaDeserialize, Debug)]
#[yaserde(
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"},
    prefix = "cal",
    rename = "filter",
)]
pub(super) struct CalendarQueryFilter {
    #[yaserde(prefix = "cal", rename = "comp-filter")]
    pub comp_filter: CompFilter,
}

#[derive(YaSerialize, YaDeserialize, Debug)]
#[yaserde(
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"},
    prefix = "cal",
    rename = "comp-filter",
)]
pub(super) struct CompFilter {
    #[yaserde(
        attribute = true,
        rename = "name",
        default = "default_comp_filter_name"
    )]
    pub name: String,

    #[yaserde(prefix = "cal", rename = "comp-filter")]
    pub comp_filters: Vec<CompFilter>,

    #[yaserde(prefix = "cal", rename = "prop-filter")]
    pub prop_filters: Vec<PropFilter>,

    #[yaserde(prefix = "cal", rename = "time-range")]
    pub time_range: Option<TimeRange>,
}

fn default_comp_filter_name() -> String {
    "".to_string()
}

#[derive(YaSerialize, YaDeserialize, Debug, Default)]
#[yaserde(
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"},
    prefix = "cal",
    rename = "prop-filter",
)]
pub(super) struct PropFilter {
    #[yaserde(attribute = true, rename = "name")]
    pub name: String,

    #[yaserde(prefix = "cal", rename = "time-range")]
    pub time_range: Option<TimeRange>,

    #[yaserde(prefix = "cal", rename = "text-match")]
    pub text_match: Option<TextMatch>,

    #[yaserde(prefix = "cal", rename = "param-filter")]
    pub param_filters: Vec<ParamFilter>,
}

#[derive(YaSerialize, YaDeserialize, Debug, Default)]
#[yaserde(
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"},
    prefix = "cal",
    rename = "param-filter",
)]
pub(super) struct ParamFilter {
    #[yaserde(attribute = true, rename = "name")]
    pub name: String,

    #[yaserde(prefix = "cal", rename = "text-match")]
    pub text_match: Option<TextMatch>,

    #[yaserde(prefix = "cal", rename = "is-not-defined")]
    pub is_not_defined: Option<IsNotDefined>,
}

#[derive(YaSerialize, YaDeserialize, Debug, Default)]
#[yaserde(
    prefix = "cal",
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"}
)]
pub(super) struct IsNotDefined;

#[derive(YaSerialize, YaDeserialize, Debug, Default)]
#[yaserde(
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"},
    prefix = "cal",
    rename = "text-match",
)]
pub(super) struct TextMatch {
    #[yaserde(attribute = true, rename = "collation", default = "default_collation")]
    pub collation: String,

    #[yaserde(
        attribute = true,
        rename = "negate-condition",
        default = "default_negate_condition"
    )]
    pub negate_condition: bool,

    #[yaserde(text = true)]
    pub text: String,
}

fn default_collation() -> String {
    "i;ascii-casemap".to_string()
}

fn default_negate_condition() -> bool {
    false
}

#[derive(YaSerialize, YaDeserialize, Debug, Default)]
#[yaserde(
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"},
    prefix = "cal",
    rename = "time-range",
)]
pub(super) struct TimeRange {
    #[yaserde(attribute = true)]
    pub start: Option<String>,

    #[yaserde(attribute = true)]
    pub end: Option<String>,
}

#[cfg(test)]
mod tests {
    use crate::caldav::propfind;

    use super::*;
    use yaserde::de::from_str;
    use yaserde::ser::to_string;

    #[test]
    fn test_calendar_query_deserialize() {
        let xml = r#"<?xml version="1.0" encoding="utf-8" ?>
            <cal:calendar-query xmlns:d="DAV:" xmlns:cal="urn:ietf:params:xml:ns:caldav">
                <d:prop>
                    <d:getetag/>
                    <cal:calendar-data/>
                </d:prop>
                <cal:filter>
                    <cal:comp-filter name="VCALENDAR">
                        <cal:comp-filter name="VEVENT">
                            <cal:time-range start="20250101T000000Z" end="20251231T235959Z" />
                        </cal:comp-filter>
                    </cal:comp-filter>
                </cal:filter>
            </cal:calendar-query>"#;

        let result: CalendarQuery = from_str(xml).unwrap();

        // Verify the prop part
        assert!(!result.prop.is_empty());

        // Verify the filter part
        let filter = result.filter.unwrap();
        assert_eq!(filter.comp_filter.name, "VCALENDAR");

        // Verify nested comp-filter
        assert!(!filter.comp_filter.comp_filters.is_empty());
        let nested_filter = &filter.comp_filter.comp_filters[0];
        assert_eq!(nested_filter.name, "VEVENT");

        // Debug print the nested filter
        println!("Nested filter: {:?}", nested_filter);

        // Verify time-range
        assert!(nested_filter.time_range.is_some());
        let time_range = nested_filter.time_range.as_ref().unwrap();
        assert_eq!(time_range.start.as_ref().unwrap(), "20250101T000000Z");
        assert_eq!(time_range.end.as_ref().unwrap(), "20251231T235959Z");
    }

    #[test]
    fn test_calendar_query_serialize() {
        // Create a calendar query with filter
        let mut query = CalendarQuery::default();

        // Add prop elements
        let mut prop = propfind::Prop::default();
        prop.getetag = Some(propfind::PropGetETag::default());
        query.prop = vec![prop];

        // Create filter structure
        let vevent_filter = CompFilter {
            name: "VEVENT".to_string(),
            time_range: Some(TimeRange {
                start: Some("20250101T000000Z".to_string()),
                end: Some("20251231T235959Z".to_string()),
            }),
            comp_filters: vec![],
            prop_filters: vec![],
        };

        let vcalendar_filter = CompFilter {
            name: "VCALENDAR".to_string(),
            comp_filters: vec![vevent_filter],
            prop_filters: vec![],
            time_range: None,
        };

        let calendar_filter = CalendarQueryFilter {
            comp_filter: vcalendar_filter,
        };

        query.filter = Some(calendar_filter);

        // Serialize to XML
        let xml = to_string(&query).unwrap();

        // Verify XML contains expected elements
        assert!(xml.contains("calendar-query"));
        assert!(xml.contains("comp-filter name=\"VCALENDAR\""));
        assert!(xml.contains("comp-filter name=\"VEVENT\""));
        assert!(xml.contains("time-range"));
        assert!(xml.contains("start=\"20250101T000000Z\""));
        assert!(xml.contains("end=\"20251231T235959Z\""));

        println!("XML IS: {}", xml);

        // Deserialize back and verify
        let deserialized: CalendarQuery = from_str(&xml).unwrap();
        assert!(deserialized.filter.is_some());
    }
}
