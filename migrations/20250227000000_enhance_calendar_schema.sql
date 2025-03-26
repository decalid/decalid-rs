-- Add timezone tables
CREATE TABLE timezones (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tzid TEXT NOT NULL UNIQUE,
    raw_data TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE timezone_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timezone_id INTEGER NOT NULL,
    rule_type TEXT NOT NULL, -- "STANDARD" or "DAYLIGHT"
    dtstart TIMESTAMP NOT NULL,
    tzoffsetfrom TEXT NOT NULL,
    tzoffsetto TEXT NOT NULL,
    rrule TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (timezone_id) REFERENCES timezones (id) ON DELETE CASCADE
);

-- Add timezone_id to calendars
ALTER TABLE calendars ADD COLUMN timezone_id INTEGER;
CREATE INDEX idx_calendars_timezone_id ON calendars (timezone_id);

-- Add reconciliation fields to event_versions
ALTER TABLE event_versions ADD COLUMN sync_status TEXT;
ALTER TABLE event_versions ADD COLUMN conflict_with INTEGER;
CREATE INDEX idx_event_versions_sync_status ON event_versions (sync_status);
CREATE INDEX idx_event_versions_conflict_with ON event_versions (conflict_with);

-- Create reconciliation_logs table
CREATE TABLE reconciliation_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id INTEGER NOT NULL,
    source_version_id INTEGER NOT NULL,
    target_version_id INTEGER NOT NULL,
    resolution TEXT NOT NULL, -- "AUTO", "MANUAL", "CONFLICT"
    resolution_notes TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (event_id) REFERENCES events (id) ON DELETE CASCADE,
    FOREIGN KEY (source_version_id) REFERENCES event_versions (id) ON DELETE CASCADE,
    FOREIGN KEY (target_version_id) REFERENCES event_versions (id) ON DELETE CASCADE
);

CREATE INDEX idx_reconciliation_logs_event_id ON reconciliation_logs (event_id);
CREATE INDEX idx_reconciliation_logs_resolution ON reconciliation_logs (resolution);

-- Create trigger to update timezones.updated_at
CREATE TRIGGER update_timezones_updated_at
AFTER UPDATE ON timezones
BEGIN
    UPDATE timezones SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- Create trigger to update calendars.updated_at
CREATE TRIGGER update_calendars_updated_at
AFTER UPDATE ON calendars
BEGIN
    UPDATE calendars SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
