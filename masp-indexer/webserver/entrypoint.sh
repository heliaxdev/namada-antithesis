#!/usr/bin/bash
set -e
echo "entered entrypoint"
until pg_isready -h 30.0.0.21 -p 5432 | grep 'accepting connections'; do
  echo "Waiting for PostgreSQL to be ready..."
  sleep 3  # Wait for 3 seconds before checking again
done
echo "PostgreSQL is now accepting connections!"

./webserver