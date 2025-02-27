# Decalid

Decalid is a Rust implementation of the decentralized calendar interaction and
distribution system. It aggregates calendar data from authoritative external
sources (via CalDAV and ICS uploads), applies user-defined transformations
through a Domain-Specific Language (DSL), and then publishes "shares" (filtered,
virtual calendars) via an API.

## Features

- **Calendar Integration**: Connect to external calendars via CalDAV or manual ICS uploads
- **Transformation Engine**: Apply filters to calendar events using a custom DSL
- **Privacy-Focused**: Create filtered views of your calendars with customized privacy settings
- **Sharing Mechanism**: Share filtered calendars with others
- **CalDAV Compatibility**: Access your shares through standard calendar clients

## Installation

### Prerequisites

- Rust (latest stable version)
- SQLite

### Building from Source

```bash
# Clone the repository
git clone https://github.com/decalid/decalid-rs.git
cd decalid-rs

# Build the project
cargo build --release

# Run the binary
./target/release/decalid-rs
```

## Usage

Decalid can be used as a server, but it also supports some administrative command-line operations.

```bash
# Run as a server
cargo run -- server --port 8080

# Import an ICS file
cargo run -- import-ics --file path/to/calendar.ics --calendar-id 1

# Create a user
cargo run -- create-user --username "username"

# List users
cargo run -- list-users

# Create a calendar
cargo run -- create-calendar --name "My Calendar" --user-id 1

# List calendars
cargo run -- list-calendars --user-id 1

# Show calendar events
cargo run -- show-calendar --calendar-id 1
```

## Testing

Run the test suite with:

```bash
cargo test
```

For integration tests:

```bash
./tests.sh
```

## Project Structure

- `/caldav`: CalDAV & ICS integration (ingestion and serving shares)
- `/transformation`: DSL engine for event transformations
- `/cache`: Caching logic
- `/api`: REST and CalDAV endpoints
- `/auth`: Passkeys, pairing, and JWT authentication
- `/telemetry`: Metrics and logging
- `/tests`: Unit and integration tests
- `/db`: Database models and operations
- `/commands`: CLI command implementations

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

Please make sure your code passes all tests and follows the project's coding style.

## License

This project is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0) - see the [LICENSE](LICENSE) file for details.
