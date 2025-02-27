use super::propfind::Response;

/// Extracts calendar data from a response
pub fn extract_calendar_data(response: &Response) -> Vec<&str> {
    let mut calendar_data = Vec::new();

    if let Some(cal_data) = &response.propstat.prop.calendar_data {
        if let Some(ref text) = cal_data.text {
            calendar_data.push(text.as_str());
        }
    }

    calendar_data
}

/// Extracts etag from a response
pub fn extract_etag(response: &Response) -> Option<&str> {
    response
        .propstat
        .prop
        .getetag
        .iter()
        .flat_map(|etag| &etag.content)
        .flat_map(|t| &t.text)
        .map(|s| s.as_str())
        .next()
}

#[cfg(test)]
mod tests {
    use yaserde::de::from_str;

    use crate::caldav::propfind::Multistatus;

    #[test]
    fn test_sync_collection_multistatus_deserialize() {
        let xml = r#"<?xml version="1.0" encoding="utf-8" ?>
        <d:multistatus xmlns:d="DAV:" xmlns:cal="urn:ietf:params:xml:ns:caldav">
            <d:sync-token>http://example.com/sync/1234</d:sync-token>
            <d:response>
                <d:href>/calendars/user/calendar/event1.ics</d:href>
                <d:propstat>
                    <d:prop>
                        <d:getetag>"etag1"</d:getetag>
                        <cal:calendar-data>BEGIN:VCALENDAR...</cal:calendar-data>
                    </d:prop>
                    <d:status>HTTP/1.1 200 OK</d:status>
                </d:propstat>
            </d:response>
        </d:multistatus>"#;

        let response: Multistatus = from_str(xml).unwrap();
        assert_eq!(
            response.sync_token.unwrap().text.unwrap(),
            "http://example.com/sync/1234"
        );
        assert_eq!(response.responses.len(), 1);
    }

    #[test]
    fn test_sync_collection_response_deserialize() {
        let xml = r#"<?xml version="1.0" encoding="utf-8" ?>
        <d:multistatus xmlns:d="DAV:" xmlns:cal="urn:ietf:params:xml:ns:caldav">
            <d:sync-token>http://example.com/sync/1234</d:sync-token>
            <d:response>
                <d:href>/calendars/user/calendar/event1.ics</d:href>
                <d:propstat>
                    <d:prop>
                        <d:getetag>"etag1"</d:getetag>
                        <cal:calendar-data>BEGIN:VCALENDAR...</cal:calendar-data>
                    </d:prop>
                    <d:status>HTTP/1.1 200 OK</d:status>
                </d:propstat>
            </d:response>
        </d:multistatus>"#;

        let response: Multistatus = from_str(xml).unwrap();
        assert_eq!(
            response.sync_token.unwrap().text.unwrap(),
            "http://example.com/sync/1234"
        );
        assert_eq!(response.responses.len(), 1);
    }
}
