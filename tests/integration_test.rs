//! Integration tests
//!
//! This module contains integration tests for the application.

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anyhow::Result;
    use chrono_tz::Tz as ChronoTz;
    use decalid::events::model::ParsedEvent;

    // This is a placeholder test that always passes
    #[test]
    fn test_sample_ics_exists() {
        assert!(Path::new("tests/sample.ics").exists());
    }

    // More integration tests will be added as the application develops

    #[test]
    fn test_floating_events_use_calendar_timezone() -> Result<()> {
        let ics = std::fs::read_to_string("tests/floating_default_timezone.ics")?;
        let mut parser = ical::IcalParser::new(ics.as_bytes());
        let mut parsed_events = Vec::new();
        let default_tz: ChronoTz = "America/New_York".parse().unwrap();

        while let Some(calendar) = parser.next() {
            let calendar = calendar?;
            for event in calendar.events {
                parsed_events.push(ParsedEvent::with_default_timezone(event, Some(default_tz))?);
            }
        }

        assert_eq!(parsed_events.len(), 3);

        let floating = parsed_events
            .iter()
            .find(|event| event.uid.as_deref() == Some("floating@example.com"))
            .expect("floating event present");
        assert_eq!(
            floating.dtstart.expect("floating dtstart").to_rfc3339(),
            "2024-03-05T14:00:00+00:00"
        );

        let localized = parsed_events
            .iter()
            .find(|event| event.uid.as_deref() == Some("localized@example.com"))
            .expect("localized event present");
        assert_eq!(
            localized.dtstart.expect("localized dtstart").to_rfc3339(),
            "2024-03-06T14:00:00+00:00"
        );

        let utc = parsed_events
            .iter()
            .find(|event| event.uid.as_deref() == Some("utc@example.com"))
            .expect("utc event present");
        assert_eq!(
            utc.dtstart.expect("utc dtstart").to_rfc3339(),
            "2024-03-07T14:00:00+00:00"
        );

        Ok(())
    }
}
