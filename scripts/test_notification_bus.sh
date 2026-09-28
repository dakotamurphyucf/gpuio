#!/bin/sh
# Deterministic freedesktop transport fixture on a private session bus; no display.
set -eu
export GPUIO_PRIVATE_BUS_TEST=1
dbus-run-session -- ./scripts/gpuio exec cargo test -p gpuio-portal \
  --locked -j "${GPUIO_JOBS:-2}" --test notifications_session -- --ignored --nocapture
dbus-run-session -- ./scripts/gpuio exec cargo test -p gpuio-native \
  --locked -j "${GPUIO_JOBS:-2}" --lib notification_linux::bus_tests \
  -- --ignored --nocapture
