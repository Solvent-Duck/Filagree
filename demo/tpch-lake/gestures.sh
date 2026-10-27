#!/bin/sh
# Start the gesture bridge in the background for the stage demo.
# Run after publish_model has opened the viewer: the bridge connects to it on start.
cd "$(dirname "$0")"
log="$PWD/.visualyze/gestures.log"
mkdir -p .visualyze
pkill -f filagree_gestures 2>/dev/null
(cd ../../gestures && nohup uv run python -m filagree_gestures --preview >"$log" 2>&1 &)

# Wait for the bridge to report it is on, or show why it stopped.
i=0
while [ $i -lt 40 ]; do
  if grep -q "Gestures on" "$log" 2>/dev/null; then
    echo "Gestures on (log: .visualyze/gestures.log)"
    exit 0
  fi
  if ! pgrep -f filagree_gestures >/dev/null; then
    echo "Gesture bridge stopped:" >&2
    tail -5 "$log" >&2
    exit 1
  fi
  sleep 0.5
  i=$((i + 1))
done
echo "Gesture bridge still starting (log: .visualyze/gestures.log)"
