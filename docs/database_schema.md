# DecalID CalDAV Server Database Schema

This document outlines the database schema design for the DecalID CalDAV server, focusing on how it handles different calendar object versions and reconciliation.

## Core Entities

### Users

The `users` table stores basic user information:

```sql
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### Calendars (VCALENDAR)

The `calendars` table represents individual calendars owned by users:

```sql
CREATE TABLE calendars (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    color TEXT,
    timezone_id INTEGER,
    etag TEXT,
    timezone TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    FOREIGN KEY (timezone_id) REFERENCES timezones (id) ON DELETE SET NULL
);
```

### Calendar Sources

The `calendar_sources` table tracks external CalDAV sources for calendars:

```sql
CREATE TABLE calendar_sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    calendar_id INTEGER NOT NULL,
    caldav_url TEXT,
    sync_token TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (calendar_id) REFERENCES calendars (id) ON DELETE CASCADE
);
```

### Events (VEVENT)

The `events` table stores the core event information:

```sql
CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    calendar_id INTEGER NOT NULL,
    current_version_id INTEGER,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (calendar_id) REFERENCES calendars (id) ON DELETE CASCADE
);
```

### Event Versions

The `event_versions` table implements versioning for events, which is crucial for reconciliation:

```sql
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
    is_all_day INTEGER NOT NULL DEFAULT 0,
    last_repeat TEXT,
    sync_status TEXT,
    conflict_with INTEGER,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_retrieved_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (event_id) REFERENCES events (id) ON DELETE CASCADE
);
```

### Event UIDs

The `event_uids` table maps events to their UIDs across different sync domains:

```sql
CREATE TABLE event_uids (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id INTEGER NOT NULL,
    uid TEXT NOT NULL,
    sync_domain TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (event_id) REFERENCES events (id) ON DELETE CASCADE,
    UNIQUE (uid, sync_domain)
);
```

### Event Attendees

The `event_attendees` table stores attendee information for events:

```sql
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
```

### Event Alarms (VALARM)

The `event_alarms` table stores alarm information for events:

```sql
CREATE TABLE event_alarms (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_version_id INTEGER NOT NULL,
    action TEXT NOT NULL,
    trigger TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (event_version_id) REFERENCES event_versions (id) ON DELETE CASCADE
);
```

### Timezones (VTIMEZONE)

The `timezones` table stores timezone information:

```sql
CREATE TABLE timezones (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tzid TEXT NOT NULL UNIQUE,
    raw_data TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### Timezone Rules

The `timezone_rules` table stores timezone rules (STANDARD and DAYLIGHT):

```sql
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
```

### Reconciliation Logs

The `reconciliation_logs` table tracks reconciliation activities:

```sql
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
```

## Versioning and Reconciliation Strategy

### Event Versioning

The schema is designed to handle multiple versions of the same event through the `event_versions` table. Each event in the `events` table can have multiple versions in the `event_versions` table, with the `current_version_id` field pointing to the active version.

Key features:
- Each version has a sequential `version` number
- The `raw_data` field stores the complete iCalendar data for the event
- The `sync_status` field tracks the synchronization status of the version
- The `conflict_with` field can reference another version that conflicts with this one

### Reconciliation Process

When synchronizing calendars from multiple sources, the system may encounter conflicting versions of the same event. The reconciliation process is tracked through the `reconciliation_logs` table:

1. When a conflict is detected, both versions are preserved in the `event_versions` table
2. The `sync_status` field is set to "CONFLICT" for the conflicting versions
3. The `conflict_with` field references the conflicting version
4. A record is created in the `reconciliation_logs` table with `resolution` set to "CONFLICT"
5. The conflict can be resolved automatically or manually:
   - Automatic resolution: Based on rules like "newest wins" or "source priority"
   - Manual resolution: User selects which version to keep or merges them

When resolved, the `reconciliation_logs` record is updated with the appropriate `resolution` value and notes.

## Indices and Performance Optimizations

The schema includes several indices to optimize common queries:

- `idx_events_current_version_id`: Fast lookup of the current version of an event
- `idx_event_versions_event_id`: Fast lookup of all versions of an event
- `idx_event_versions_dtstart`, `idx_event_versions_dtend`: Fast date range queries
- `idx_event_uids_uid_sync_domain`: Fast lookup of events by UID and sync domain
- `idx_event_versions_sync_status`: Fast lookup of events by sync status
- `idx_event_versions_conflict_with`: Fast lookup of conflicting event versions
- `idx_reconciliation_logs_event_id`: Fast lookup of reconciliation logs by event
- `idx_reconciliation_logs_resolution`: Fast lookup of reconciliation logs by resolution type

## Relationship Diagram

```
users
 ↑
 |
 +-- calendars --+-- timezones --+-- timezone_rules
      ↑          |
      |          +-- events --+-- event_versions --+-- event_attendees
      |                       |                    |
      |                       |                    +-- event_alarms
      |                       |                    |
      |                       |                    +-- freebusy
      |                       |
      |                       +-- event_uids
      |                       |
      |                       +-- reconciliation_logs
      |
      +-- calendar_sources
      |
      +-- calendar_shares
```

## Design Decisions

1. **Separate Event and EventVersion Tables**: This separation allows for efficient versioning and conflict resolution without duplicating the entire event record.

2. **Raw Data Storage**: Each event version stores the complete iCalendar data in `raw_data`, ensuring no information is lost during parsing or synchronization.

3. **Flexible Timezone Handling**: The dedicated `timezones` and `timezone_rules` tables allow for proper handling of complex timezone definitions.

4. **Reconciliation Tracking**: The `reconciliation_logs` table provides an audit trail of conflict resolution decisions.

5. **Efficient UID Lookup**: The `event_uids` table with its unique constraint on `(uid, sync_domain)` ensures efficient lookup of events by their UID across different sync domains.

6. **Comprehensive Indexing**: Indices are defined for all common query patterns to ensure optimal performance.
