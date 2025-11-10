use anyhow::Result;

use decalid::db::Db;
use decalid::timezone::ParsedTimezone;

/// List all timezones in the database
pub async fn list_timezones(db: &Db) -> Result<()> {
    let timezones = db.timezones().list_all().await?;

    if timezones.is_empty() {
        println!("No timezones found");
        return Ok(());
    }

    println!("Timezones:");
    for tz in timezones {
        println!("- ID: {}, TZID: {}", tz.id, tz.tzid);
    }

    Ok(())
}

/// Import a timezone from an ICS file
pub async fn import_timezone(db: &Db, file_path: &str) -> Result<()> {
    // Read the ICS file
    let ics_content = std::fs::read_to_string(file_path)?;

    // Parse the ICS content
    let reader = ical::IcalParser::new(ics_content.as_bytes());

    let mut imported_count = 0;

    for cal_result in reader {
        let cal = cal_result?;

        for timezone in cal.timezones {
            let parsed_timezone = ParsedTimezone::new(timezone)?;
            let saved_timezone = db.timezones().save_timezone(&parsed_timezone).await?;

            println!("Imported timezone: {}", saved_timezone.tzid);
            imported_count += 1;
        }
    }

    if imported_count == 0 {
        println!("No timezones found in the file");
    } else {
        println!("Successfully imported {imported_count} timezone(s)");
    }

    Ok(())
}

/// Set the timezone for a calendar
pub async fn set_calendar_timezone(db: &Db, calendar_id: i64, tzid: &str) -> Result<()> {
    // Check if the calendar exists
    let calendar = db.admin().get_calendar_by_id(calendar_id).await?;

    // Find the timezone
    let timezone = db.timezones().get_by_tzid(tzid).await?;

    match timezone {
        Some(tz) => {
            // Associate the timezone with the calendar
            db.timezones()
                .associate_with_calendar(calendar_id, tz.id)
                .await?;
            println!(
                "Set timezone '{}' for calendar '{}'",
                tz.tzid, calendar.name
            );
            Ok(())
        }
        None => {
            println!("Timezone '{tzid}' not found");
            Err(anyhow::anyhow!("Timezone not found"))
        }
    }
}

/// Show details of a timezone
pub async fn show_timezone(db: &Db, tzid: &str) -> Result<()> {
    let timezone = db.timezones().get_by_tzid(tzid).await?;

    match timezone {
        Some(tz) => {
            println!("Timezone: {}", tz.tzid);
            println!("ID: {}", tz.id);
            println!("Created: {}", tz.created_at);
            println!("Updated: {}", tz.updated_at);

            // Get rules
            let rules = db.timezones().get_rules(tz.id).await?;

            if !rules.is_empty() {
                println!("\nRules:");
                for rule in rules {
                    println!("- Type: {}", rule.rule_type);
                    println!("  Start: {}", rule.dtstart);
                    println!("  From: {}", rule.tzoffsetfrom);
                    println!("  To: {}", rule.tzoffsetto);
                    if let Some(rrule) = &rule.rrule {
                        println!("  Recurrence: {rrule}");
                    }
                    println!();
                }
            }

            println!("\nRaw Data:");
            println!("{}", tz.raw_data);

            Ok(())
        }
        None => {
            println!("Timezone '{tzid}' not found");
            Err(anyhow::anyhow!("Timezone not found"))
        }
    }
}
