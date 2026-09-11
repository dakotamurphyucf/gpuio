#!/bin/sh
# Xvfb supplies an X server; Openbox supplies window mapping/focus/WM protocols.
set -eu
openbox --sm-disable > .cache/ci/openbox.log 2>&1 &
GPUIO_WM_PID=$!
trap 'kill "$GPUIO_WM_PID" 2>/dev/null || true' EXIT INT TERM
GPUIO_WM_READY=false
for attempt in 1 2 3 4 5 6 7 8 9 10; do
  if xprop -root _NET_SUPPORTING_WM_CHECK | grep -q 'window id'; then
    GPUIO_WM_READY=true
    break
  fi
  sleep 1
done
if [ "$GPUIO_WM_READY" != true ]; then
  cat .cache/ci/openbox.log >&2
  exit 1
fi
xrandr --verbose > .cache/ci/xrandr.log
xprop -root > .cache/ci/x11-root.log
timeout 90 ./scripts/gpuio smoke --self-test
timeout 90 ./scripts/gpuio smoke --two-windows
timeout 90 _build/default/examples/bridge/main.exe
    timeout 90 _build/default/examples/view_api/main.exe --self-test
    timeout 90 ./scripts/gpuio exec cargo test --locked -p gpuio-native --features native-tests --test native_ui
