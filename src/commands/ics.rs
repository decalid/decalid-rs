use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use sqlx::SqlitePool;
use std::str::FromStr;

use crate::models::{Calendar, Event, EventVersion};

pub async fn import_ics(pool: &SqlitePool, calendar_id: i64, file_path: &str) -> Result<()> {
    // Create or get the calendar
    let calendar = sqlx::query_as::<_, Calendar>("SELECT * FROM calendars WHERE id = ?")
        .bind(calendar_id)
        .fetch_one(pool)
        .await?;
    println!("Using calendar: {:?}", calendar);
    // Read the ICS file
    let ics_content = std::fs::read_to_string(file_path)?;
    let reader = ical::IcalParser::new(ics_content.as_bytes());

    // Process each calendar component
    for cal_result in reader {
        let cal = cal_result?;
        for event in cal.events {
            // Create a new event
            let event_record = sqlx::query_as::<_, Event>(
                "INSERT INTO events (calendar_id, created_at, updated_at) VALUES (?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) RETURNING *"
            )
            .bind(calendar.id)
            .fetch_one(pool)
            .await?;

            // Extract event properties
            let summary = event
                .properties
                .iter()
                .find(|p| p.name == "SUMMARY")
                .and_then(|p| p.value.clone());

            let description = event
                .properties
                .iter()
                .find(|p| p.name == "DESCRIPTION")
                .and_then(|p| p.value.clone());

            let dtstart = event
                .properties
                .iter()
                .find(|p| p.name == "DTSTART")
                .and_then(ical_property_to_datetime);

            let dtend = event
                .properties
                .iter()
                .find(|p| p.name == "DTEND")
                .and_then(ical_property_to_datetime);

            let uid = event
                .properties
                .iter()
                .find(|p| p.name == "UID")
                .and_then(|p| p.value.clone());

            // Serialize the event to ICS format
            let event_raw_data = serialize_event(&event);
            // Start a transaction
            let mut transaction = pool.begin().await?;

            // Create a new event version
            let event_version = sqlx::query_as::<_, EventVersion>(
                "INSERT INTO event_versions (event_id, version, summary, description, dtstart, dtend, raw_data, created_at, last_retrieved_at) 
                 VALUES (?, 1, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) 
                 RETURNING *"
            )
            .bind(event_record.id)
            .bind(summary)
            .bind(description)
            .bind(dtstart)
            .bind(dtend)
            .bind(&event_raw_data)
            .fetch_one(&mut *transaction)
            .await?;

            // Update the event with the current version
            sqlx::query("UPDATE events SET current_version_id = ? WHERE id = ?")
                .bind(event_version.id)
                .bind(event_record.id)
                .execute(&mut *transaction)
                .await?;

            // Create event UID
            if let Some(uid_value) = uid {
                sqlx::query(
                    "INSERT INTO event_uids (event_id, uid, sync_domain, created_at) 
                     VALUES (?, ?, ?, CURRENT_TIMESTAMP)",
                )
                .bind(event_record.id)
                .bind(uid_value)
                .bind(calendar.id.to_string()) // Using calendar_id as sync_domain
                .execute(&mut *transaction)
                .await?;
            }

            // Rollback the transaction
            transaction.commit().await?;

            println!("Imported event: {:?}", event_version);
        }
    }

    Ok(())
}

fn ical_property_to_datetime(
    property: &ical::property::Property,
) -> Option<DateTime<Tz>> {
    let value = property.value.clone();

    // If we a TZID parameter, we need to remove it and keep the timezone
    let tz_str = property
        .params
        .clone()
        .into_iter()
        .flatten()
        .find(|(k, _)| k == "TZID")
        .map(|(_, v)| v.join(","))
        .unwrap_or_else(|| "UTC".to_string());
    let tz = chrono_tz::Tz::from_str(&tz_str).ok();

    let value = value.map(|value| {
        // If the value is a date, add a time component
        if value.chars().count() == 8 {
            format!("{}T000000", value)
        } else {
            value
        }
    });

    value.map(|value| {
        println!("I got the following pre-parse: {:?} (tz={:?})", value, tz);
        let dt = NaiveDateTime::parse_from_str(&value, "%Y%m%dT%H%M%S").unwrap();
        if let Some(tz) = tz {
            tz.from_local_datetime(&dt).single().unwrap()
        } else {
            chrono_tz::UTC.from_local_datetime(&dt).single().unwrap()
        }
    })
}

fn serialize_event(event: &ical::parser::ical::component::IcalEvent) -> String {
    let mut output = String::from("BEGIN:VEVENT\r\n");

    for property in &event.properties {
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

    output.push_str("END:VEVENT\r\n");
    output
}
