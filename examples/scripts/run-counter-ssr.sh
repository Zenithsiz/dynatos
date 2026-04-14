#!/usr/bin/env bash

set -e

trap "pkill -P $$" SIGHUP SIGINT SIGTERM

$(cd counter-ssr-frontend && trunk serve) &
FRONTEND_PID=$!

$(cd counter-ssr-backend && cargo watch -x 'run') &
BACKEND_PID=$!

wait "$FRONTEND_PID" "$BACKEND_PID"
