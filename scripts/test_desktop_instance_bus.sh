#!/bin/sh
# No display and no changes to the developer's session bus or environment.
set -eu
export GPUIO_PRIVATE_BUS_TEST=1
exec dbus-run-session -- ./scripts/gpuio exec cargo test -p gpuio-portal \
  --locked -j "${GPUIO_JOBS:-2}" --test instance_session -- --ignored --nocapture
