#!/bin/bash

# This script generates ICS files for testing purposes

generate_simple_event() {
  local uid=$1
  local summary=$2
  local description=$3
  local location=$4
  local start_date=$5
  local end_date=$6
  
  cat << EOF
BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//decalid-rs//CalDAV Client//EN
BEGIN:VEVENT
UID:${uid}
DTSTAMP:20250318T215800Z
DTSTART:${start_date}
DTEND:${end_date}
SUMMARY:${summary}
DESCRIPTION:${description}
LOCATION:${location}
STATUS:CONFIRMED
END:VEVENT
END:VCALENDAR
EOF
}

generate_recurring_event() {
  local uid=$1
  local summary=$2
  local description=$3
  local location=$4
  local start_date=$5
  local end_date=$6
  local rrule=$7
  
  cat << EOF
BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//decalid-rs//CalDAV Client//EN
BEGIN:VEVENT
UID:${uid}
DTSTAMP:20250318T215800Z
DTSTART:${start_date}
DTEND:${end_date}
RRULE:${rrule}
SUMMARY:${summary}
DESCRIPTION:${description}
LOCATION:${location}
STATUS:CONFIRMED
END:VEVENT
END:VCALENDAR
EOF
}

generate_allday_event() {
  local uid=$1
  local summary=$2
  local description=$3
  local location=$4
  local start_date=$5
  local end_date=$6
  
  cat << EOF
BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//decalid-rs//CalDAV Client//EN
BEGIN:VEVENT
UID:${uid}
DTSTAMP:20250318T215800Z
DTSTART;VALUE=DATE:${start_date}
DTEND;VALUE=DATE:${end_date}
SUMMARY:${summary}
DESCRIPTION:${description}
LOCATION:${location}
STATUS:CONFIRMED
END:VEVENT
END:VCALENDAR
EOF
}

# Export functions
export -f generate_simple_event
export -f generate_recurring_event
export -f generate_allday_event
