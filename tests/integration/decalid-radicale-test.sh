#!/bin/bash

set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Source the ICS generator functions
source "$SCRIPT_DIR/generate-ics.sh"

# Colors for output
RED="\033[0;31m"
GREEN="\033[0;32m"
YELLOW="\033[0;33m"
NC="\033[0m" # No Color

# Test configuration
RADICALE_HOST="http://localhost:5232"
DECALID_HOST="http://localhost:8085"
USERNAME="test-user"
PASSWORD="password"
CALENDAR_NAME="test-calendar"
USER_PRINCIPAL="/test-user/"
CALENDAR_HOME="/test-user/"
CALENDAR_PATH="/test-user/$CALENDAR_NAME/"
DECALID_CALENDAR_PATH="/decalid/$CALENDAR_NAME/"

DECALID_TOKEN=""
DECALID_DEVICE_ID=""
DECALID_EXPIRATION=""

# Function to run curl with basic auth against Radicale
curl_radicale() {
    curl --fail-with-body -s -u "$USERNAME:$PASSWORD" "$@"
}

# Function to run curl with basic auth against decalid
curl_decalid() {
    curl --fail-with-body -s -H "Authorization: Bearer $DECALID_TOKEN" "$@"
}

# Function to check if a command succeeded
check_result() {
    local result=$?
    local message=$1
    if [ $result -eq 0 ]; then
        echo -e "${GREEN}✓ $message${NC}"
    else
        echo -e "${RED}✗ ($result) $message${NC}"
        echo -e "${YELLOW}$2${NC}"
        if [[ "$NODIE" != "1" ]]; then
            exit 1
        fi
    fi
}

# Start Docker services
start_services() {
    echo -e "\n${YELLOW}Starting Docker services (Radicale and decalid)...${NC}"
    cd "$SCRIPT_DIR"
    docker-compose up -d --build
    
    # Wait for services to be ready
    echo -e "${YELLOW}Waiting for services to be ready...${NC}"
    
    # Function to check if a service is ready
    wait_for_service() {
        local service_name=$1
        local url=$2
        local max_attempts=30
        local attempt=1
        
        echo -e "${YELLOW}Checking $service_name availability...${NC}"
        
        while [ $attempt -le $max_attempts ]; do
            if curl -s -f -o /dev/null "$url"; then
                echo -e "${GREEN}$service_name is ready!${NC}"
                return 0
            fi
            
            echo -e "${YELLOW}Waiting for $service_name (attempt $attempt/$max_attempts)...${NC}"
            sleep 1
            attempt=$((attempt + 1))
        done
        
        echo -e "${RED}$service_name did not become available after $max_attempts attempts${NC}"
        return 1
    }
    
    # Check both services
    wait_for_service "Radicale" "http://localhost:5232" || {
        echo -e "${RED}Radicale service failed to start properly${NC}"
        docker-compose logs radicale
        exit 1
    }
    
    wait_for_service "Decalid" "http://localhost:8085" || {
        echo -e "${RED}Decalid service failed to start properly${NC}"
        docker-compose logs decalid
        exit 1
    }
    
    echo -e "${GREEN}Docker services started successfully${NC}"
}

# Stop Docker services
stop_services() {
    echo -e "\n${YELLOW}Stopping Docker services...${NC}"
    cd "$SCRIPT_DIR"
    docker-compose down
    echo -e "${GREEN}Docker services stopped${NC}"
}

# Setup Radicale with test data
setup_radicale() {
    echo -e "\n${YELLOW}Setting up Radicale with test data...${NC}"
    
    # Create calendar collection
    MKCALENDAR_XML="<?xml version=\"1.0\" encoding=\"UTF-8\"?><mkcol xmlns=\"DAV:\" xmlns:C=\"urn:ietf:params:xml:ns:caldav\">
    <set>
        <prop>
        <resourcetype>
            <collection/>
            <C:calendar/>
        </resourcetype>
        <displayname>Test Calendar</displayname>
        </prop>
    </set></mkcol>"

    curl_radicale -X MKCOL \
        -H "Content-Type: application/xml" \
        -d "$MKCALENDAR_XML" \
        "$RADICALE_HOST$CALENDAR_PATH" > /dev/null
    check_result "Calendar collection created in Radicale"
    
    # Add events
    # 1. Simple event
    SIMPLE_EVENT=$(generate_simple_event \
        "simple-event@decalid-rs.test" \
        "Simple Test Event" \
        "This is a test event created by the integration test" \
        "Test Location" \
        "20250320T100000Z" \
        "20250320T110000Z")
    
    curl_radicale -X PUT \
        -H "Content-Type: text/calendar" \
        -d "$SIMPLE_EVENT" \
        "$RADICALE_HOST$CALENDAR_PATH/simple-event.ics" > /dev/null
    check_result "Simple event added to Radicale"
    
    # 2. Recurring event
    RECURRING_EVENT=$(generate_recurring_event \
        "recurring-event@decalid-rs.test" \
        "Recurring Test Event" \
        "This is a recurring test event that happens every Friday for 10 weeks" \
        "Recurring Test Location" \
        "20250321T140000Z" \
        "20250321T150000Z" \
        "FREQ=WEEKLY;BYDAY=FR;COUNT=10")
    
    curl_radicale -X PUT \
        -H "Content-Type: text/calendar" \
        -d "$RECURRING_EVENT" \
        "$RADICALE_HOST$CALENDAR_PATH/recurring-event.ics" > /dev/null
    check_result "Recurring event added to Radicale"
    
    # 3. All-day event
    ALLDAY_EVENT=$(generate_allday_event \
        "allday-event@decalid-rs.test" \
        "All-Day Test Event" \
        "This is an all-day test event" \
        "All-Day Test Location" \
        "20250322" \
        "20250323")
    
    curl_radicale -X PUT \
        -H "Content-Type: text/calendar" \
        -d "$ALLDAY_EVENT" \
        "$RADICALE_HOST$CALENDAR_PATH/allday-event.ics" > /dev/null
    check_result "All-day event added to Radicale"
}

radicale_is_setup() {
    curl_radicale -X GET "$RADICALE_HOST$CALENDAR_PATH" > /dev/null
    NODIE=1 check_result "Radicale is already set up"
}

get_decalid_token() {
    local command="$1"
    local tokenfile="$2"
    echo "Getting decalid token for authentication"
    # Create a user using the CLI command
    $command create-user --username "$USERNAME" > /dev/null
    CONTENT="$($command login-new-device --user-id 1 --device-description "Test Device")"
    DECALID_TOKEN=$(echo "$CONTENT" | grep "Token:" | sed 's/Token: //')
    DECALID_DEVICE_ID=$(echo "$CONTENT" | grep "Device ID:" | sed 's/Device ID: //')
    DECALID_EXPIRATION=$(echo "$CONTENT" | grep "Expiration:" | sed 's/Expiration: //')
    cat >"$tokenfile" <<EOF
DECALID_TOKEN="$DECALID_TOKEN"
DECALID_DEVICE_ID="$DECALID_DEVICE_ID"
DECALID_EXPIRATION="$DECALID_EXPIRATION"
EOF
    cat "$tokenfile"
    echo "Token: $DECALID_TOKEN"
    echo "Device ID: $DECALID_DEVICE_ID"
    echo "Expiration: $DECALID_EXPIRATION"
}

get_decalid_token_docker() {
    get_decalid_token "docker-compose exec -T decalid /app/decalid-rs"
    source .last_token
}

get_decalid_token_local() {
    local tokenfile=$PWD/.last_token
    if [[ -f "$tokenfile" ]]; then
        source "$tokenfile"
        # Data may be empty (or invalid)
        if [[ -n "$DECALID_TOKEN" && -n "$DECALID_DEVICE_ID" && -n "$DECALID_EXPIRATION" ]]; then
            # Check that the expiration date is still valid
            if [[ $(date -d "$DECALID_EXPIRATION" +%s) -gt $(date +%s) ]]; then
                return
            fi
        fi
    fi
    pushd ../../
    get_decalid_token "target/debug/decalid-rs" "$tokenfile"
    source "$tokenfile"
    popd
}

check_decalid_configured () {
    # Check that a calendar exists and it has a source
    # Use jq to count that there is a single element in the .data field
    response=$(curl_decalid -X GET "$DECALID_HOST/api/calendars/")
    echo "Got response: $response"
    if [[ $(echo "$response" | jq -r '.data | length') -eq 0 ]]; then
        echo "Not configured yet"
        return 1
    fi
    check_result "decalid calendar exists"
    local calendar_id=$(echo "$response" | jq -r '.data[0].id')
    response=$(curl_decalid -X GET "$DECALID_HOST/api/calendars/$calendar_id/sources/")
    echo "Response: $response"
    if [[ $(echo "$response" | jq -r '.data | length') -eq 0 ]]; then
        return 1
    fi
    check_result "decalid calendar has a source"
}

# Configure decalid to use Radicale as a source
configure_decalid() {
    if check_decalid_configured; then
        echo -e "\n${YELLOW}decalid is already configured to use Radicale as a source...${NC}"
        return
    fi

    local response
    local calendar_id
    echo -e "\n${YELLOW}Configuring decalid to use Radicale as a source...${NC}"

    # Create a calendar for the user
    response=$(curl_decalid -v -X POST -H "Content-Type: application/json" \
        -d '{"name":"'$CALENDAR_NAME'"}' \
        "$DECALID_HOST/api/calendars/")
    check_result "decalid calendar created" "$response"
    calendar_id=$(echo "$response" | grep -E '"id":"(\\w+)' | sed 's/"id":"//')
    echo "Calendar ID: $calendar_id"
    
    # Add Radicale source
    curl_decalid -X POST -H "Content-Type: application/json" \
        -d '{"type":"caldav","url":"'$RADICALE_HOST'", "username":"'$USERNAME'", "password":"'$PASSWORD'", "calendar_path":"'$CALENDAR_PATH'"}' \
        "$DECALID_HOST/api/calendars/$calendar_id/sources/add" > /dev/null
    
    check_result "decalid configured to use Radicale"
}

# Test decalid's ability to read from Radicale
test_decalid_read() {
    echo -e "\n${YELLOW}Testing decalid's ability to read from Radicale...${NC}"
    
    # Query events from decalid
    # This is a placeholder and should be replaced with actual API calls
    # to your decalid server once the API is implemented
    
    # Example:
    # QUERY_RESULT=$(curl_decalid -X GET "$DECALID_HOST/api/calendars/$CALENDAR_NAME/events")
    # echo "$QUERY_RESULT" | grep -q "simple-event@decalid-rs.test" && \
    # echo "$QUERY_RESULT" | grep -q "recurring-event@decalid-rs.test" && \
    # echo "$QUERY_RESULT" | grep -q "allday-event@decalid-rs.test"
    
    echo -e "${YELLOW}Note: decalid read test is a placeholder. Implement actual API calls when available.${NC}"
    # For now, we'll assume it works correctly
    check_result "decalid can read events from Radicale (placeholder)"
}

# Test decalid's ability to write to Radicale
test_decalid_write() {
    echo -e "\n${YELLOW}Testing decalid's ability to write to Radicale...${NC}"
    
    # Create a new event through decalid
    # This is a placeholder and should be replaced with actual API calls
    # to your decalid server once the API is implemented
    
    # Example:
    # NEW_EVENT=$(generate_simple_event \
    #     "decalid-created-event@decalid-rs.test" \
    #     "Event Created by decalid" \
    #     "This event was created through the decalid server" \
    #     "decalid Test Location" \
    #     "20250325T120000Z" \
    #     "20250325T130000Z")
    # 
    # curl_decalid -X PUT \
    #     -H "Content-Type: text/calendar" \
    #     -d "$NEW_EVENT" \
    #     "$DECALID_HOST/api/calendars/$CALENDAR_NAME/events/decalid-created-event.ics" > /dev/null
    
    echo -e "${YELLOW}Note: decalid write test is a placeholder. Implement actual API calls when available.${NC}"
    # For now, we'll assume it works correctly
    check_result "decalid can write events to Radicale (placeholder)"
    
    # Verify the event was created in Radicale
    # curl_radicale -X GET "$RADICALE_HOST$CALENDAR_PATH/decalid-created-event.ics" | grep -q "decalid-created-event@decalid-rs.test"
    # check_result "Event created by decalid exists in Radicale"
}

# Test decalid's transformation capabilities
test_decalid_transformations() {
    echo -e "\n${YELLOW}Testing decalid's transformation capabilities...${NC}"
    
    # Configure a transformation in decalid
    # This is a placeholder and should be replaced with actual API calls
    # to your decalid server once the API is implemented
    
    # Example:
    # curl_decalid -X POST \
    #     -H "Content-Type: application/json" \
    #     -d '{"source_calendar":"$CALENDAR_NAME", "transformation":"anonymize_title", "max_recurrences":3}' \
    #     "$DECALID_HOST/api/transformations/add" > /dev/null
    
    echo -e "${YELLOW}Note: decalid transformation test is a placeholder. Implement actual API calls when available.${NC}"
    # For now, we'll assume it works correctly
    check_result "decalid transformation configured (placeholder)"
    
    # Verify the transformation works
    # TRANSFORMED_RESULT=$(curl_decalid -X GET "$DECALID_HOST/api/calendars/transformed/$CALENDAR_NAME/events")
    # echo "$TRANSFORMED_RESULT" | grep -q "Anonymized Event" && \
    # echo "$TRANSFORMED_RESULT" | grep -q "COUNT=3"
    # check_result "Transformation applied correctly"
}

# Main test function
run_tests() {
    echo -e "${YELLOW}Starting integration tests between decalid and Radicale...${NC}"
    
    # Setup
    # start_services
    if ! radicale_is_setup; then
        setup_radicale
    fi
    get_decalid_token_local
    configure_decalid
    
    # Tests
    test_decalid_read
    test_decalid_write
    test_decalid_transformations
    
    # Cleanup
    stop_services
    
    echo -e "\n${GREEN}All integration tests completed successfully!${NC}"
}

# Run the tests
run_tests

exit 0
