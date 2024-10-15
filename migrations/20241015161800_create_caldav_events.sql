CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE calendars (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    color TEXT,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

CREATE TABLE calendar_sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    calendar_id INTEGER NOT NULL,
    caldav_url TEXT,
    sync_token TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (calendar_id) REFERENCES calendars (id) ON DELETE CASCADE
);

CREATE TABLE calendar_shares (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    calendar_id INTEGER NOT NULL,
    url_slug TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (calendar_id) REFERENCES calendars (id) ON DELETE CASCADE
);


CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    calendar_id INTEGER NOT NULL,
    current_version_id INTEGER,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (calendar_id) REFERENCES calendars (id) ON DELETE CASCADE
);

-- Create event_versions table
CREATE TABLE event_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id INTEGER NOT NULL,
    version INTEGER NOT NULL,
    summary TEXT,
    description TEXT,
    dtstart TIMESTAMP,
    dtend TIMESTAMP,
    duration TEXT,
    rrule TEXT,
    rdate TEXT,
    exdate TEXT,
    status TEXT,
    organizer TEXT,
    location TEXT,
    url TEXT,
    class TEXT,
    priority INTEGER,
    transp TEXT,
    sequence INTEGER,
    raw_data TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_retrieved_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (event_id) REFERENCES events (id) ON DELETE CASCADE
);

-- Create event_uids table
CREATE TABLE event_uids (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id INTEGER NOT NULL,
    uid TEXT NOT NULL,
    sync_domain TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (event_id) REFERENCES events (id) ON DELETE CASCADE,
    UNIQUE (uid, sync_domain)
);

-- Create event_attendees table
CREATE TABLE event_attendees (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_version_id INTEGER NOT NULL,
    attendee TEXT NOT NULL,
    role TEXT,
    partstat TEXT,
    rsvp TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (event_version_id) REFERENCES event_versions (id) ON DELETE CASCADE
);

-- Create event_alarms table
CREATE TABLE event_alarms (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_version_id INTEGER NOT NULL,
    action TEXT NOT NULL,
    trigger TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (event_version_id) REFERENCES event_versions (id) ON DELETE CASCADE
);

-- Create freebusy table
CREATE TABLE freebusy (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_version_id INTEGER NOT NULL,
    fbtype TEXT NOT NULL,
    start_time TIMESTAMP NOT NULL,
    end_time TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (event_version_id) REFERENCES event_versions (id) ON DELETE CASCADE
);

-- Create indexes
CREATE INDEX idx_events_current_version_id ON events (current_version_id);
CREATE INDEX idx_event_versions_event_id ON event_versions (event_id);
CREATE INDEX idx_event_versions_last_retrieved_at ON event_versions (last_retrieved_at);
CREATE INDEX idx_event_versions_dtstart ON event_versions (dtstart);
CREATE INDEX idx_event_versions_dtend ON event_versions (dtend);
CREATE INDEX idx_event_versions_summary ON event_versions (summary);
CREATE INDEX idx_event_uids_event_id ON event_uids (event_id);
CREATE INDEX idx_event_uids_uid_sync_domain ON event_uids (uid, sync_domain);
CREATE INDEX idx_event_attendees_event_version_id ON event_attendees (event_version_id);
CREATE INDEX idx_event_alarms_event_version_id ON event_alarms (event_version_id);
CREATE INDEX idx_freebusy_event_version_id ON freebusy (event_version_id);
CREATE INDEX idx_freebusy_start_time ON freebusy (start_time);
CREATE INDEX idx_freebusy_end_time ON freebusy (end_time);

-- Create trigger to update events.updated_at
CREATE TRIGGER update_events_updated_at
AFTER UPDATE ON events
BEGIN
    UPDATE events SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
