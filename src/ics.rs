use anyhow::Result;

use crate::{
    db::{
        models::{Calendar, EventVersion, Timezone},
        Db,
    },
    events::model::ParsedEvent,
    timezone::{parse_chrono_tz, resolve_timezone_reference, ParsedTimezone},
};

pub async fn import_ics(db: &Db, calendar_id: i64, file_path: &str) -> Result<()> {
    // Check the calendar exists
    let _ = db.admin().get_calendar_by_id(calendar_id).await?;

    // Read the ICS file
    let ics_content = std::fs::read_to_string(file_path)?;

    // Import the ICS content
    import_ics_data(db, calendar_id, file_path, &ics_content).await
}

/// Import ICS data directly from a string
pub async fn import_ics_data(
    db: &Db,
    calendar_id: i64,
    url: &str,
    ics_content: &str,
) -> Result<()> {
    // Check the calendar exists
    let calendar = db.admin().get_calendar_by_id(calendar_id).await?;

    let reader = ical::IcalParser::new(ics_content.as_bytes());

    // Process each calendar component
    let events_db = db.events(calendar_id);
    let timezone_db = db.timezones();

    for cal_result in reader {
        let cal = cal_result?;

        // Process timezone components first
        let mut primary_timezone_id = calendar.timezone_id;

        for timezone in cal.timezones {
            let parsed_timezone = ParsedTimezone::new(timezone)?;
            let saved_timezone = timezone_db.save_timezone(&parsed_timezone).await?;

            // If this is the first timezone and the calendar doesn't have a timezone set,
            // associate it with the calendar
            if primary_timezone_id.is_none() {
                timezone_db
                    .associate_with_calendar(calendar_id, saved_timezone.id)
                    .await?;
                primary_timezone_id = Some(saved_timezone.id);
            }
        }

        let default_timezone = if let Some(timezone_id) = primary_timezone_id {
            timezone_db
                .get_by_id(timezone_id)
                .await?
                .map(|tz| resolve_timezone_reference(&tz.tzid))
                .or_else(|| {
                    let tz = calendar.timezone.trim();
                    (!tz.is_empty()).then(|| resolve_timezone_reference(tz))
                })
        } else {
            let tz = calendar.timezone.trim();
            (!tz.is_empty()).then(|| resolve_timezone_reference(tz))
        }
        .map(|tzid| parse_chrono_tz(&tzid))
        .transpose()?;

        // Now process events
        for event in cal.events {
            // Create a new event
            let event = ParsedEvent::new_with_default_timezone(event, default_timezone)?;
            let event = ParsedEvent {
                url: Some(url.to_string()),
                ..event
            };
            let event_version = events_db.create(event).await?;
            println!(
                "Imported event: {}",
                event_version.summary.unwrap_or_default()
            );
        }
    }

    Ok(())
}

pub async fn convert_to_icalendar(
    db: &Db,
    calendar: &Calendar,
    events: &Vec<EventVersion>,
) -> String {
    let mut buffer =
        "PRODID:-//Santiago Saavedra//DECALID V0//EN\r\nVERSION:1.0\r\nBEGIN:VCALENDAR\r\n"
            .to_string();

    // Add timezone if the calendar has one
    if let Some(timezone_id) = calendar.timezone_id {
        if let Ok(Some(timezone)) = db.timezones().get_by_id(timezone_id).await {
            // Add the raw timezone data
            buffer.push_str(&timezone.raw_data);
            if !buffer.ends_with("\r\n") {
                buffer.push_str("\r\n");
            }
        }
    }

    for event in events {
        buffer.push_str("BEGIN:VEVENT\r\n");
        buffer.push_str(&event.raw_data);
        if !buffer.ends_with("\r\n") {
            buffer.push_str("\r\n");
        }
        buffer.push_str("END:VEVENT\r\n");
    }

    buffer.push_str("END:VCALENDAR\r\n");
    buffer
}

/// Get a timezone by TZID
pub async fn get_timezone_by_tzid(db: &Db, tzid: &str) -> Result<Option<Timezone>> {
    // Resolve the timezone ID (handle RFC 7809 references)
    let resolved_tzid = crate::timezone::resolve_timezone_reference(tzid);

    // Look up the timezone
    db.timezones().get_by_tzid(&resolved_tzid).await
}

#[cfg(test)]
mod tests {
    use crate::events::model::ParsedEvent;
    use anyhow::Result;

    #[tokio::test]
    async fn test_small_ics_parse() -> Result<()> {
        let mut events = 0;
        // This text was taken from a manual test with Radicale
        let text = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//decalid-rs//CalDAV Client//EN\r\nBEGIN:VEVENT\r\nUID:simple-event@decalid-rs.test\r\nDTSTART:20250320T100000Z\r\nDTEND:20250320T110000Z\r\nDESCRIPTION:This is a test event created by the integration test\r\nDTSTAMP:20250318T215800Z\r\nLOCATION:Test Location\r\nSTATUS:CONFIRMED\r\nSUMMARY:Simple Test Event\r\nEND:VEVENT\r\nEND:VCALENDAR";

        let parser = ical::IcalParser::new(text.as_bytes());

        for cal_result in parser {
            let cal = cal_result?;

            // Now process events
            for event in cal.events {
                events += 1;
                // Create a new event
                let event = ParsedEvent::new(event)?;
                let event_version = event;
                println!(
                    "Imported event: {}",
                    event_version.summary.unwrap_or_default()
                );
            }
        }
        assert!(events == 1);
        Ok(())
    }
}
