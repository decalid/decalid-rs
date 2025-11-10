use anyhow::Result;
use sqlx::sqlite::SqlitePool;

use crate::db::Db;
use crate::timezone::timezone::ParsedTimezone;

// Sample ICS data with VTIMEZONE
const SAMPLE_ICS_WITH_TIMEZONE: &str = r#"BEGIN:VCALENDAR
PRODID:-//Google Inc//Google Calendar 70.9054//EN
VERSION:2.0
CALSCALE:GREGORIAN
METHOD:PUBLISH
BEGIN:VTIMEZONE
TZID:America/New_York
X-LIC-LOCATION:America/New_York
BEGIN:DAYLIGHT
TZOFFSETFROM:-0500
TZOFFSETTO:-0400
TZNAME:EDT
DTSTART:19700308T020000
RRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=2SU
END:DAYLIGHT
BEGIN:STANDARD
TZOFFSETFROM:-0400
TZOFFSETTO:-0500
TZNAME:EST
DTSTART:19701101T020000
RRULE:FREQ=YEARLY;BYMONTH=11;BYDAY=1SU
END:STANDARD
END:VTIMEZONE
BEGIN:VEVENT
DTSTART;TZID=America/New_York:20250301T090000
DTEND;TZID=America/New_York:20250301T100000
DTSTAMP:20250227T034400Z
UID:test-event-with-timezone@example.com
CREATED:20250227T034400Z
DESCRIPTION:Test event with timezone
LAST-MODIFIED:20250227T034400Z
LOCATION:New York
SEQUENCE:0
STATUS:CONFIRMED
SUMMARY:Test Event with Timezone
TRANSP:OPAQUE
END:VEVENT
END:VCALENDAR"#;

// Helper function to normalize ICS data by removing carriage returns and empty lines
fn normalize_ics(ics: &str) -> String {
    ics.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test]
async fn test_parse_timezone() -> Result<()> {
    // Parse the ICS content
    let reader = ical::IcalParser::new(SAMPLE_ICS_WITH_TIMEZONE.as_bytes());

    let mut found_timezone = false;

    for cal_result in reader {
        let cal = cal_result?;

        for timezone in cal.timezones {
            let parsed_timezone = ParsedTimezone::new(timezone)?;

            // Verify the timezone data
            assert_eq!(parsed_timezone.tzid, "America/New_York");
            assert_eq!(parsed_timezone.standard_rules.len(), 1);
            assert_eq!(parsed_timezone.daylight_rules.len(), 1);

            // Verify standard rule
            let standard = &parsed_timezone.standard_rules[0];
            assert_eq!(standard.tzoffsetfrom, "-0400");
            assert_eq!(standard.tzoffsetto, "-0500");

            // Verify daylight rule
            let daylight = &parsed_timezone.daylight_rules[0];
            assert_eq!(daylight.tzoffsetfrom, "-0500");
            assert_eq!(daylight.tzoffsetto, "-0400");

            found_timezone = true;
        }
    }

    assert!(found_timezone, "No timezone was found in the test data");

    Ok(())
}

#[tokio::test]
async fn test_timezone_db_storage() -> Result<()> {
    // Create an in-memory SQLite database
    let pool = SqlitePool::connect("sqlite::memory:").await?;

    // Run migrations to set up the schema
    sqlx::migrate!().run(&pool).await?;

    let db = Db::new(pool);

    // Parse the ICS content
    let reader = ical::IcalParser::new(SAMPLE_ICS_WITH_TIMEZONE.as_bytes());

    for cal_result in reader {
        let cal = cal_result?;

        for timezone in cal.timezones {
            let parsed_timezone = ParsedTimezone::new(timezone)?;

            // Save timezone to database
            let saved_timezone = db.timezones().save_timezone(&parsed_timezone).await?;

            // Retrieve timezone from database by TZID
            let retrieved_timezone = db
                .timezones()
                .get_by_tzid(&parsed_timezone.tzid)
                .await?
                .expect("Timezone should exist in database");

            // Verify the retrieved timezone matches the original
            assert_eq!(retrieved_timezone.tzid, parsed_timezone.tzid);

            // Get the rules for the saved timezone
            let rules = db.timezones().get_rules(saved_timezone.id).await?;

            // Verify we have both standard and daylight rules
            assert_eq!(rules.len(), 2);

            // Verify the rules match what we expect
            let standard_rule = rules
                .iter()
                .find(|r| r.rule_type == "STANDARD")
                .expect("Should have a STANDARD rule");
            let daylight_rule = rules
                .iter()
                .find(|r| r.rule_type == "DAYLIGHT")
                .expect("Should have a DAYLIGHT rule");

            assert_eq!(standard_rule.tzoffsetfrom, "-0400");
            assert_eq!(standard_rule.tzoffsetto, "-0500");
            assert_eq!(daylight_rule.tzoffsetfrom, "-0500");
            assert_eq!(daylight_rule.tzoffsetto, "-0400");

            // Verify that the raw ICS data matches the original
            // First, normalize both the original and stored ICS to remove carriage returns and empty lines
            let original_ics = normalize_ics(
                &SAMPLE_ICS_WITH_TIMEZONE
                    .lines()
                    .skip_while(|line| !line.starts_with("BEGIN:VTIMEZONE"))
                    .take_while(|line| !line.starts_with("BEGIN:VEVENT"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
            let stored_ics = normalize_ics(&retrieved_timezone.raw_data);

            assert_eq!(
                original_ics, stored_ics,
                "Stored ICS data should match original"
            );
        }
    }

    Ok(())
}
