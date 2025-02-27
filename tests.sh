#!/bin/bash

set -e

do_ () {
    echo "You need to select a command."
}

do_build_db () {

sqlx database reset --database-url sqlite:db.sqlite
cargo run -- create-user -u test
cargo run -- create-calendar -u 1 -n testcal
cargo run -- import-ics -c 1 -f ./tests/sample.ics

cargo run -- create-share-root -s xtest -o 1
cargo run -- add-calendar-to-share -s xtest -c 1 -d "Added from CLI"

# Show results
cargo run -- show-calendar -c 1
}

do_caldav_propfind_test () {
    COLL_URL=http://localhost:8000/xtest
    curl -v -X PROPFIND "$COLL_URL" -H "Depth: 1" -H "Content-Type: application/xml" -d '<?xml version="1.0" encoding="utf-8"?><propfind xmlns="DAV:"><allprop/></propfind>'
    echo "-- With specific namespace in d: --"
    curl -v -X PROPFIND "$COLL_URL" -H "Depth: 1" -H "Content-Type: application/xml" -d '<?xml version="1.0" encoding="utf-8"?><d:propfind xmlns:d="DAV:"><d:allprop/></d:propfind>'
    echo "-- With specific namespace in xii: --"
    curl -v -X PROPFIND "$COLL_URL" -H "Depth: 1" -H "Content-Type: application/xml" -d '<?xml version="1.0" encoding="utf-8"?><xii:propfind xmlns:xii="DAV:"><xii:allprop/></xii:propfind>'

    curl -X PROPFIND "$COLL_URL" -H "Depth: 1" -H "Content-Type: application/xml" -d '<?xml version="1.0" encoding="utf-8"?>
<propfind xmlns="DAV:">
  <propname/>
</propfind>'

    curl -X PROPFIND "$COLL_URL" -H "Depth: 1" -H "Content-Type: application/xml" -d '<?xml version="1.0" encoding="utf-8"?><propfind xmlns="DAV:"><prop><displayname/></prop></propfind>'
    curl -X PROPFIND "$COLL_URL" -H "Depth: 1" -H "Content-Type: application/xml" -d '<?xml version="1.0" encoding="utf-8"?><propfind xmlns="DAV:"><prop></prop></propfind>'
    curl -X PROPFIND "$COLL_URL" -H "Depth: 1" -H "Content-Type: application/xml" -d '<?xml version="1.0" encoding="utf-8"?><propfind xmlns="DAV:"></propfind>'
    curl -X PROPFIND "$COLL_URL" -H "Depth: 1" -H "Content-Type: application/xml" -d '<?xml version="1.0" encoding="utf-8"?><propfind xmlns="DAV:"><allprop/><propname/></propfind>'

    CAL_URL=$COLL_URL/L_-P6pgT3D
}




do_$1