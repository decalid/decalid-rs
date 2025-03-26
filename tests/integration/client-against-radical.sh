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

echo -e "${YELLOW}Starting integration tests against Radicale...${NC}"

# Ensure docker-compose is available
if ! command -v docker-compose &> /dev/null; then
    echo -e "${RED}docker-compose is not installed. Please install it to run these tests.${NC}"
    exit 1
fi

# Start Radicale and decalid servers using docker-compose
echo -e "${YELLOW}Starting Radicale and decalid servers...${NC}"
cd "$SCRIPT_DIR"
docker-compose up -d

# Wait for services to be ready
echo -e "${YELLOW}Waiting for services to be ready...${NC}"
sleep 5

# Run the tests
echo -e "${YELLOW}Running CalDAV tests...${NC}"
bash "$SCRIPT_DIR/caldav-tests.sh"
TEST_RESULT=$?

# Clean up
echo -e "${YELLOW}Cleaning up...${NC}"
docker-compose down

# Report results
if [ $TEST_RESULT -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
else
    echo -e "${RED}Tests failed!${NC}"
fi

exit $TEST_RESULT