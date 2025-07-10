#!/bin/sh

set -ue

# Start the ClamAV daemon
echo "Starting ClamAV daemon..."
clamd &

# Run the Rust application
echo "Starting Rust application..."
virus-scan


