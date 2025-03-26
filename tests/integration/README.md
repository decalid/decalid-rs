# Integration Tests for decalid-rs

This directory contains integration tests for testing decalid-rs against real CalDAV servers.

## Test Overview

The integration tests are designed to verify that decalid-rs can properly interact with standard CalDAV servers. The tests focus on the following capabilities:

1. Basic CalDAV operations (authentication, calendar creation, event CRUD)
2. Support for different event types (simple, recurring, all-day)
3. Calendar aggregation and transformation features

## Available Tests

### 1. Client Against Radicale

Tests the decalid client against the Radicale CalDAV server.

- `client-against-radical.sh`: Main test script that orchestrates the testing process
- `docker-compose.yml`: Docker Compose configuration for Radicale and decalid
- `caldav-tests.sh`: Script containing the actual CalDAV test operations
- `generate-ics.sh`: Helper script for generating ICS files with different event types
- `decalid-radicale-test.sh`: Comprehensive test for decalid-Radicale integration

## Prerequisites

- Docker and Docker Compose
- curl
- bash

## Running the Tests

### Testing Against Radicale

To run the basic CalDAV client tests against Radicale:

```bash
./tests/integration/client-against-radical.sh
```

This will:
1. Start Radicale and decalid servers using Docker Compose
2. Run a series of CalDAV operations against Radicale
3. Clean up when done

### Testing decalid-Radicale Integration

To test the integration between decalid and Radicale:

```bash
./tests/integration/decalid-radicale-test.sh
```

This will:
1. Set up a Radicale server with test data
2. Start the decalid server via Docker
3. Configure decalid to use Radicale as a source
4. Test decalid's ability to read from and write to Radicale
5. Test decalid's transformation capabilities

## Test Structure

The tests follow a modular design with single responsibility per component:

- **Docker services**: Both Radicale and decalid run in Docker containers
- **Setup scripts**: Prepare the environment (start services, create test data)
- **Test scripts**: Execute the actual test operations
- **Helper scripts**: Provide reusable functions (e.g., ICS generation)
- **Cleanup scripts**: Ensure proper teardown of test resources

## Docker Configuration

The integration tests use Docker to ensure consistent environments:

- **Radicale**: Uses the `tomsquest/docker-radicale:latest` image with authentication configured
- **decalid**: Built from the project's Dockerfile with proper configuration

The Docker Compose setup includes:

1. A shared network for service communication
2. Volume mounts for persistent data and configuration
3. Environment variables for proper service configuration

## Extending the Tests

To add new test cases:

1. For new event types: Add generator functions to `generate-ics.sh`
2. For new CalDAV operations: Add test functions to `caldav-tests.sh`
3. For new integration scenarios: Create new test scripts following the existing pattern

## Notes

- The integration tests with decalid contain placeholders for API calls that should be replaced with actual implementation once the API is finalized.
- The tests are designed to be idempotent and to clean up after themselves, but manual cleanup may be necessary if a test fails unexpectedly.
