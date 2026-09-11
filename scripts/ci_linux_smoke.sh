#!/bin/sh
set -eu
mkdir -p .cache/ci
GPUIO_RUNTIME_DIR=$(mktemp -d)
export XDG_RUNTIME_DIR="$GPUIO_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"
export VK_ICD_FILENAMES=$(python3 -c 'import glob; print(glob.glob("/usr/share/vulkan/icd.d/*lvp*.json")[0])')
export EIO_BACKEND=posix

case "${1:-}" in
  x11)
    unset WAYLAND_DISPLAY
    xvfb-run -a -s '-screen 0 1280x1024x24' \
      timeout 90 ./scripts/gpuio smoke --self-test
    xvfb-run -a -s '-screen 0 1280x1024x24' \
      timeout 90 ./scripts/gpuio smoke --two-windows
    ;;
  wayland)
    unset DISPLAY
    export WAYLAND_DISPLAY=wayland-gpuio
    weston --backend=headless-backend.so --use-pixman --socket="$WAYLAND_DISPLAY" \
      --idle-time=0 --width=1280 --height=1024 > .cache/ci/weston.log 2>&1 &
    GPUIO_WESTON_PID=$!
    trap 'kill "$GPUIO_WESTON_PID" 2>/dev/null || true' EXIT INT TERM
    for attempt in 1 2 3 4 5 6 7 8 9 10; do
      if [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]; then break; fi
      sleep 1
    done
    if [ ! -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]; then
      cat .cache/ci/weston.log >&2
      exit 1
    fi
    timeout 90 ./scripts/gpuio smoke --self-test
    timeout 90 ./scripts/gpuio smoke --two-windows
    ;;
  *) echo 'Expected x11 or wayland' >&2; exit 2 ;;
esac
