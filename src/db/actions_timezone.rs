use anyhow::Result;

use crate::timezone::ParsedTimezone;
use crate::db::models::{Timezone, TimezoneRule};
use crate::db::Db;

pub struct TimezoneDb<'a> {
    pub(super) db: &'a Db,
}

impl<'a> TimezoneDb<'a> {
    /// Get a timezone by its TZID
    pub async fn get_by_tzid(&self, tzid: &str) -> Result<Option<Timezone>> {
        let timezone = sqlx::query_as::<_, Timezone>("SELECT * FROM timezones WHERE tzid = ?")
            .bind(tzid)
            .fetch_optional(&self.db.0)
            .await?;
        
        Ok(timezone)
    }
    
    /// Get timezone rules for a timezone
    pub async fn get_rules(&self, timezone_id: i64) -> Result<Vec<TimezoneRule>> {
        let rules = sqlx::query_as::<_, TimezoneRule>("SELECT * FROM timezone_rules WHERE timezone_id = ? ORDER BY dtstart")
            .bind(timezone_id)
            .fetch_all(&self.db.0)
            .await?;
        
        Ok(rules)
    }
    
    /// Save a timezone to the database
    pub async fn save_timezone(&self, timezone: &ParsedTimezone) -> Result<Timezone> {
        // Check if the timezone already exists
        if let Some(existing) = self.get_by_tzid(&timezone.tzid).await? {
            // Update existing timezone
            let updated = sqlx::query_as::<_, Timezone>(
                "UPDATE timezones SET raw_data = ? WHERE id = ? RETURNING *"
            )
            .bind(&timezone.raw_data)
            .bind(existing.id)
            .fetch_one(&self.db.0)
            .await?;
            
            // Delete old rules
            sqlx::query("DELETE FROM timezone_rules WHERE timezone_id = ?")
                .bind(existing.id)
                .execute(&self.db.0)
                .await?;
            
            // Insert new rules
            for rule in timezone.rules_to_db_models(existing.id) {
                self.save_rule(&rule).await?;
            }
            
            Ok(updated)
        } else {
            // Insert new timezone
            let db_timezone = timezone.to_db_model();
            
            let inserted = sqlx::query_as::<_, Timezone>(
                "INSERT INTO timezones (tzid, raw_data) VALUES (?, ?) RETURNING *"
            )
            .bind(&db_timezone.tzid)
            .bind(&db_timezone.raw_data)
            .fetch_one(&self.db.0)
            .await?;
            
            // Insert rules
            for rule in timezone.rules_to_db_models(inserted.id) {
                self.save_rule(&rule).await?;
            }
            
            Ok(inserted)
        }
    }
    
    /// Save a timezone rule
    async fn save_rule(&self, rule: &TimezoneRule) -> Result<TimezoneRule> {
        let inserted = sqlx::query_as::<_, TimezoneRule>(
            "INSERT INTO timezone_rules 
            (timezone_id, rule_type, dtstart, tzoffsetfrom, tzoffsetto, rrule) 
            VALUES (?, ?, ?, ?, ?, ?) 
            RETURNING *"
        )
        .bind(rule.timezone_id)
        .bind(&rule.rule_type)
        .bind(rule.dtstart)
        .bind(&rule.tzoffsetfrom)
        .bind(&rule.tzoffsetto)
        .bind(&rule.rrule)
        .fetch_one(&self.db.0)
        .await?;
        
        Ok(inserted)
    }
    
    /// List all timezones
    pub async fn list_all(&self) -> Result<Vec<Timezone>> {
        let timezones = sqlx::query_as::<_, Timezone>("SELECT * FROM timezones ORDER BY tzid")
            .fetch_all(&self.db.0)
            .await?;
        
        Ok(timezones)
    }
    
    /// Get timezone by ID
    pub async fn get_by_id(&self, id: i64) -> Result<Option<Timezone>> {
        let timezone = sqlx::query_as::<_, Timezone>("SELECT * FROM timezones WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.db.0)
            .await?;
        
        Ok(timezone)
    }
    
    /// Associate a timezone with a calendar
    pub async fn associate_with_calendar(&self, calendar_id: i64, timezone_id: i64) -> Result<()> {
        sqlx::query("UPDATE calendars SET timezone_id = ? WHERE id = ?")
            .bind(timezone_id)
            .bind(calendar_id)
            .execute(&self.db.0)
            .await?;
        
        Ok(())
    }
}
