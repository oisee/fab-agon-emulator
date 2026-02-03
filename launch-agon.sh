#!/bin/bash
# Launch split Agon emulator (eZ80 + text VDP) for terminal/SSH use

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SDCARD="${1:-$SCRIPT_DIR/sdcard}"
SOCKET="/tmp/agon-vdp-$$.sock"

cleanup() {
    kill $EZ80_PID 2>/dev/null
    rm -f "$SOCKET"
}
trap cleanup EXIT

# Start eZ80 server
"$SCRIPT_DIR/target/release/agon-ez80" --socket "$SOCKET" --sdcard "$SDCARD" &
EZ80_PID=$!

# Wait for socket to appear
for i in {1..20}; do
    [ -S "$SOCKET" ] && break
    sleep 0.1
done

if [ ! -S "$SOCKET" ]; then
    echo "Error: eZ80 failed to start"
    exit 1
fi

# Start text VDP (foreground - handles terminal I/O)
"$SCRIPT_DIR/target/release/agon-vdp-cli" --socket "$SOCKET"
