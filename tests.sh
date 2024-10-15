#!/bin/bash

set -e

sqlx database reset --database-url sqlite:db.sqlite
cargo run -- create-user -u test
cargo run -- create-calendar -u 1 -n testcal
cargo run -- import-ics -c 1 -f ./tests/sample.ics

# Show results
cargo run -- show-calendar -c 1
