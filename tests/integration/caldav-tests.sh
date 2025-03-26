#!/bin/bash

set -e
set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors for output
RED="\033[0;31m"
GREEN="\033[0;32m"
YELLOW="\033[0;33m"
NC="\033[0m" # No Color

# Test configuration
RADICALE_HOST="http://localhost:5232"
USERNAME="test-user"
PASSWORD="password"
CALENDAR_NAME="test-calendar"
USER_PRINCIPAL="/test-user/"
CALENDAR_HOME="/test-user/"
CALENDAR_PATH="/test-user/$CALENDAR_NAME/"

# Function to run curl with basic auth
curl_auth() {
    curl -s -u "$USERNAME:$PASSWORD" "$@"
}

# Function to check if a command succeeded
check_result() {
    local result=$?
    local message=$1
    if [ $result -eq 0 ]; then
        echo -e "${GREEN}✓ $message${NC}"
    else
        echo -e "${RED}✗ $message${NC}"
        exit 1
    fi
}

# Test 1: Check if Radicale is running
echo -e "\n${YELLOW}Test 1: Checking if Radicale is running...${NC}"
curl -s "$RADICALE_HOST" > /dev/null
check_result "Radicale is running"

# Test 2: Authenticate with Radicale
echo -e "\n${YELLOW}Test 2: Authenticating with Radicale...${NC}"
curl_auth "$RADICALE_HOST" > /dev/null
check_result "Authentication successful"

# Test 3: Create a calendar collection
echo -e "\n${YELLOW}Test 3: Creating a calendar collection...${NC}"
MKCALENDAR_XML="<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<mkcol xmlns=\"DAV:\" xmlns:C=\"urn:ietf:params:xml:ns:caldav\">
  <set>
    <prop>
      <resourcetype>
        <collection/>
        <C:calendar/>
      </resourcetype>
      <displayname>Test Calendar</displayname>
    </prop>
  </set>
</mkcol>"

curl_auth -X MKCOL \
    -H "Content-Type: application/xml" \
    -d "$MKCALENDAR_XML" \
    "$RADICALE_HOST$CALENDAR_PATH" > /dev/null
check_result "Calendar collection created"

# Test 4: Add a simple event
echo -e "\n${YELLOW}Test 4: Adding a simple event...${NC}"
SIMPLE_EVENT="BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//decalid-rs//CalDAV Client//EN
BEGIN:VEVENT
UID:simple-event@decalid-rs.test
DTSTAMP:20250318T215800Z
DTSTART:20250320T100000Z
DTEND:20250320T110000Z
SUMMARY:Simple Test Event
DESCRIPTION:This is a test event created by the integration test
LOCATION:Test Location
STATUS:CONFIRMED
END:VEVENT
END:VCALENDAR"

curl_auth -X PUT \
    -H "Content-Type: text/calendar" \
    -d "$SIMPLE_EVENT" \
    "$RADICALE_HOST$CALENDAR_PATH/simple-event.ics" > /dev/null
check_result "Simple event added"

# Test 5: Add a recurring event
echo -e "\n${YELLOW}Test 5: Adding a recurring event...${NC}"
RECURRING_EVENT="BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//decalid-rs//CalDAV Client//EN
BEGIN:VEVENT
UID:recurring-event@decalid-rs.test
DTSTAMP:20250318T215800Z
DTSTART:20250321T140000Z
DTEND:20250321T150000Z
RRULE:FREQ=WEEKLY;BYDAY=FR;COUNT=10
SUMMARY:Recurring Test Event
DESCRIPTION:This is a recurring test event that happens every Friday for 10 weeks
LOCATION:Recurring Test Location
STATUS:CONFIRMED
END:VEVENT
END:VCALENDAR"

curl_auth -X PUT \
    -H "Content-Type: text/calendar" \
    -d "$RECURRING_EVENT" \
    "$RADICALE_HOST$CALENDAR_PATH/recurring-event.ics" > /dev/null
check_result "Recurring event added"

# Test 6: Add an all-day event
echo -e "\n${YELLOW}Test 6: Adding an all-day event...${NC}"
ALLDAY_EVENT="BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//decalid-rs//CalDAV Client//EN
BEGIN:VEVENT
UID:allday-event@decalid-rs.test
DTSTAMP:20250318T215800Z
DTSTART;VALUE=DATE:20250322
DTEND;VALUE=DATE:20250323
SUMMARY:All-Day Test Event
DESCRIPTION:This is an all-day test event
LOCATION:All-Day Test Location
STATUS:CONFIRMED
END:VEVENT
END:VCALENDAR"

curl_auth -X PUT \
    -H "Content-Type: text/calendar" \
    -d "$ALLDAY_EVENT" \
    "$RADICALE_HOST$CALENDAR_PATH/allday-event.ics" > /dev/null
check_result "All-day event added"

# Test 7: Retrieve events using calendar-query
echo -e "\n${YELLOW}Test 7: Retrieving events using calendar-query...${NC}"
QUERY_XML="<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<C:calendar-query xmlns:D=\"DAV:\" xmlns:C=\"urn:ietf:params:xml:ns:caldav\">
  <D:prop>
    <D:getetag/>
    <C:calendar-data/>
  </D:prop>
  <C:filter>
    <C:comp-filter name=\"VCALENDAR\">
      <C:comp-filter name=\"VEVENT\">
        <C:time-range start=\"20250301T000000Z\" end=\"20250401T000000Z\"/>
      </C:comp-filter>
    </C:comp-filter>
  </C:filter>
</C:calendar-query>"

QUERY_RESULT=$(curl_auth -X REPORT \
    -H "Content-Type: application/xml" \
    -H "Depth: 1" \
    -d "$QUERY_XML" \
    "$RADICALE_HOST$CALENDAR_PATH")

echo "$QUERY_RESULT" | grep -q "simple-event@decalid-rs.test" && \
echo "$QUERY_RESULT" | grep -q "recurring-event@decalid-rs.test" && \
echo "$QUERY_RESULT" | grep -q "allday-event@decalid-rs.test"
check_result "All events retrieved successfully"

# Test 8: Modify an event
echo -e "\n${YELLOW}Test 8: Modifying an event...${NC}"
MODIFIED_EVENT="BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//decalid-rs//CalDAV Client//EN
BEGIN:VEVENT
UID:simple-event@decalid-rs.test
DTSTAMP:20250318T215800Z
DTSTART:20250320T100000Z
DTEND:20250320T110000Z
SUMMARY:Modified Test Event
DESCRIPTION:This event has been modified by the integration test
LOCATION:Modified Test Location
STATUS:CONFIRMED
END:VEVENT
END:VCALENDAR"

curl_auth -X PUT \
    -H "Content-Type: text/calendar" \
    -d "$MODIFIED_EVENT" \
    "$RADICALE_HOST$CALENDAR_PATH/simple-event.ics" > /dev/null
check_result "Event modified"

# Test 9: Delete an event
echo -e "\n${YELLOW}Test 9: Deleting an event...${NC}"
curl_auth -X DELETE "$RADICALE_HOST$CALENDAR_PATH/allday-event.ics" > /dev/null
check_result "Event deleted"

# Test 10: Verify event deletion
echo -e "\n${YELLOW}Test 10: Verifying event deletion...${NC}"
DELETE_VERIFY=$(curl_auth -X REPORT \
    -H "Content-Type: application/xml" \
    -H "Depth: 1" \
    -d "$QUERY_XML" \
    "$RADICALE_HOST$CALENDAR_PATH")

if echo "$DELETE_VERIFY" | grep -q "allday-event@decalid-rs.test"; then
    echo -e "${RED}✗ Event was not deleted${NC}"
    exit 1
else
    echo -e "${GREEN}✓ Event deletion verified${NC}"
fi

echo -e "\n${GREEN}All CalDAV tests completed successfully!${NC}"
exit 0
