#!/bin/sh
set -e

# Create data directory if it doesn't exist
mkdir -p /app/data

# Handle shutdown signals properly
trap_handler() {
    echo "Received shutdown signal, exiting gracefully..."
    kill -TERM "$child" 2>/dev/null
    wait "$child"
    exit 0
}

trap trap_handler INT TERM

# Run the decalid server with the provided arguments
echo "Starting decalid server with arguments: $@"
exec /app/decalid-rs "$@"
