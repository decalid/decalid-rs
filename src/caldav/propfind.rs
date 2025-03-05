use yaserde::{YaDeserialize, YaSerialize};


#[derive(YaSerialize, YaDeserialize, Debug, Default)]
pub(super) struct Empty;


#[derive(YaSerialize, YaDeserialize, Debug, Default)]
#[yaserde(
    flatten = true,
)]
pub(super) struct Content {
    #[yaserde(text = true)]
    pub text: Option<String>,
}

#[derive(YaSerialize, YaDeserialize, Debug, Default)]
#[yaserde(
    flatten = true,
)]
pub(super) struct ContentCdata {
    #[yaserde(cdata = true)]
    pub text: String,
}

// REQUEST
#[derive(YaSerialize, YaDeserialize, Debug)]
#[yaserde(
    prefix = "d",
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"}
)]
pub(super) struct Propfind {
    #[yaserde(prefix = "d")]
    pub prop: Vec<Prop>,
    #[yaserde(prefix = "d")]
    pub allprop: Option<AllProp>,
    #[yaserde(prefix = "d")]
    pub propname: Option<PropName>,
}

#[derive(YaSerialize, YaDeserialize, Debug)]
#[yaserde(
    prefix = "d",
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"}
)]
pub(super) struct AllProp;

#[derive(YaSerialize, YaDeserialize, Debug, Default)]
#[yaserde(
    prefix = "d",
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"}
)]
pub(super) struct PropName;

// RESPONSE

#[derive(YaSerialize, YaDeserialize, Debug)]
#[yaserde(
    prefix = "d",
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"},
    rename = "multistatus"
)]
pub(super) struct Multistatus {

    // Used for synchronization on <DAV:sync-token> element
    #[yaserde(prefix = "d", rename = "sync-token")]
    pub sync_token: Option<Content>,

    #[yaserde(prefix = "d", rename = "response")]
    pub responses: Vec<Response>,
}

#[derive(YaSerialize, YaDeserialize, Debug)]
#[yaserde(
    prefix = "d",
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"}
)]
pub(super) struct Response {
    #[yaserde(prefix = "d")]
    pub href: String,
    #[yaserde(prefix = "d")]
    pub propstat: Propstat,
}

#[derive(YaSerialize, YaDeserialize, Debug)]
#[yaserde(
    prefix = "d",
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"}
)]
pub(super) struct Propstat {
    #[yaserde(prefix = "d")]
    pub prop: Prop,
    #[yaserde(prefix = "d")]
    pub status: String,
}

#[derive(YaSerialize, YaDeserialize, Debug, Default)]
#[yaserde(
    prefix = "d",
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"}
)]
pub(super) struct CalendarDescription {
    #[yaserde(attribute = true, prefix = "xml")]
    pub lang: String,

    #[yaserde(text = true)]
    pub content: String,
}

#[derive(YaSerialize, YaDeserialize, Debug, Default)]
#[yaserde(
    prefix = "d",
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav", "ext" = "https://decalid.com/ns/calendar-0"}
)]
pub(super) struct Prop {
    #[yaserde(prefix = "d")]
    pub displayname: Option<PropDisplayName>,
    #[yaserde(prefix = "ext", rename = "calendar-color")]
    pub calendar_color: Option<PropCalendarColor>,
    #[yaserde(prefix = "cal", rename = "calendar-description")]
    pub calendar_description: Option<PropCalendarDescription>,

    #[yaserde(prefix = "d")]
    pub resourcetype: Option<PropResourceType>,
    #[yaserde(prefix = "d", rename = "getetag")]
    pub getetag: Option<PropGetETag>,
    #[yaserde(prefix = "cal", rename = "supported-calendar-component-set")]
    pub supported_calendar_component_set: Option<PropSupportedCalendarComponentSet>,
    #[yaserde(prefix = "cal", rename = "calendar-timezone", cdata = true, default = default_prop_calendar_timezone)] // Only in collections
    pub calendar_timezone: String,
    #[yaserde(prefix = "cal", rename = "supported-calendar-data")]
    pub supported_calendar_data: Option<PropSupportedCalendarData>,
    #[yaserde(prefix = "cal", rename = "calendar-data")] // Only in calendars (not in collections)
    pub calendar_data: Option<PropCalendarData>,
}
impl Prop {
    pub(crate) fn propnames() -> Prop {
        Prop {
            displayname: Some(PropDisplayName { content: None }),
            calendar_color: Some(PropCalendarColor { content: None }),
            calendar_description: Some(PropCalendarDescription { content: None }),
            resourcetype: Some(PropResourceType { calendar: None, collection: None }),
            getetag: Some(PropGetETag { content: None }),
            supported_calendar_component_set: None,
            calendar_timezone: "".to_string(), // TODO: Fix
            supported_calendar_data: None,
            calendar_data: None,
        }
    }
}

fn default_prop_calendar_timezone() -> String {
    "".to_string()
}


#[derive(YaSerialize, YaDeserialize, Debug)]
#[yaserde(
    prefix = "cal",
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"}
)]
pub(super) struct PropSupportedCalendarData {
    #[yaserde(prefix = "cal", rename = "calendar-data")]
    pub calendar_data: Option<PropCalendarData>,
}

#[derive(YaSerialize, YaDeserialize, Debug)]
#[yaserde(
    prefix = "cal",
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"},
)]
pub(super) struct PropCalendarData {
    #[yaserde(attribute = true, rename = "content-type")]
    pub content_type: Option<String>,
    #[yaserde(attribute = true)]
    pub version: Option<String>,

    #[yaserde(text = true)]
    pub text: Option<String>,
}

fn default_calendardata_content_type() -> String {
    "text/calendar".to_string()
}

fn default_calendardata_version() -> String {
    "2.0".to_string()
}

impl Default for PropCalendarData {
    fn default() -> Self {
        PropCalendarData {
            content_type: Some("text/calendar".to_string()),
            version: Some("2.0".to_string()),
            text: None,
        }
    }
}

#[derive(YaSerialize, YaDeserialize, Debug, Default)]
#[yaserde(
    prefix = "d",
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"}
)]
pub(super) struct PropResourceType {
    #[yaserde(prefix = "cal")]
    pub calendar: Option<Empty>,
    #[yaserde(prefix = "d")]
    pub collection: Option<Empty>,
}

#[derive(YaSerialize, YaDeserialize, Debug, Default)]
#[yaserde(
    prefix = "d",
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"}
)]
pub(super) struct PropGetCTag {
    #[yaserde(prefix = "d")]
    pub content: Option<Content>,
}


#[derive(YaSerialize, YaDeserialize, Debug, Default)]
#[yaserde(
    prefix = "d",
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"}
)]
pub(super) struct PropGetETag {
    #[yaserde(prefix = "d")]
    pub content: Option<Content>,
}

#[derive(YaSerialize, YaDeserialize, Debug, Default)]
#[yaserde(
    prefix = "d",
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"}
)]
pub(super) struct PropSupportedCalendarComponentSet {
    #[yaserde(prefix = "cal", rename = "comp")]
    pub components: Vec<PropSupportedCalendarComponent>,
}

impl From<Vec<PropSupportedCalendarComponent>> for PropSupportedCalendarComponentSet {
    fn from(components: Vec<PropSupportedCalendarComponent>) -> Self {
        PropSupportedCalendarComponentSet { components }
    }
}

#[derive(YaSerialize, YaDeserialize, Debug, Default, Clone)]
#[yaserde(
    prefix = "cal",
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"}
)]
pub(super) struct PropSupportedCalendarComponent {
    #[yaserde(attribute = true)]
    pub name: String,
}


#[derive(YaSerialize, YaDeserialize, Debug, Default)]
#[yaserde(
    prefix = "d",
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"}
)]
pub(super) struct PropDisplayName {
    #[yaserde(prefix = "d")]
    pub content: Option<Content>,
}

#[derive(YaSerialize, YaDeserialize, Debug, Default)]
#[yaserde(
    prefix = "d",
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"}
)]
pub(super) struct PropCalendarColor {
    #[yaserde(prefix = "d")]
    pub content: Option<Content>,
}

#[derive(YaSerialize, YaDeserialize, Debug, Default)]
#[yaserde(
    prefix = "d",
    namespaces = {"d" = "DAV:", "cal" = "urn:ietf:params:xml:ns:caldav"},
)]
pub(super) struct PropCalendarDescription {
    #[yaserde(prefix = "d")]
    pub content: Option<Content>,
}

impl From<String> for Content {
    fn from(s: String) -> Self {
        Content { text: Some(s) }
    }
}

impl From<String> for ContentCdata {
    fn from(s: String) -> Self {
        ContentCdata { text: s }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_propname_deserialize_with_empty_element() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<d:propfind xmlns:d="DAV:">
    <d:propname/>
</d:propfind>"#;

        let result: Propfind = yaserde::de::from_str(xml).unwrap();
        assert!(result.propname.is_some());
        assert!(result.prop.is_empty());
        assert!(result.allprop.is_none());
    }

    #[test]
    fn test_propname_deserialize_with_allprop() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<d:propfind xmlns:d="DAV:">
    <d:allprop/>
</d:propfind>"#;

        let result: Propfind = yaserde::de::from_str(xml).unwrap();
        assert!(result.propname.is_none());
        assert!(result.prop.is_empty());
        assert!(result.allprop.is_some());
    }

    #[test]
    fn test_propname_deserialize_with_prop() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<d:propfind xmlns:d="DAV:" xmlns:cal="urn:ietf:params:xml:ns:caldav" xmlns:ext="https://decalid.com/ns/calendar-0">
    <d:prop>
        <d:displayname/>
        <ext:calendar-color/>
        <cal:calendar-description/> 
    </d:prop>
</d:propfind>"#;

        let result: Propfind = yaserde::de::from_str(xml).unwrap();
        assert!(result.propname.is_none());
        assert!(!result.prop.is_empty());
        assert!(result.allprop.is_none());

        let prop = result.prop.first().unwrap();
        assert!(prop.displayname.is_some());
        assert!(prop.calendar_color.is_some());
        assert!(prop.calendar_description.is_some());
    }
}
