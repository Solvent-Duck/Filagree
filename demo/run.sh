#!/usr/bin/env bash
# One command for the stage demo: build, open the viewer maximized on the
# TPC-H model and tour, start gestures, then ask in the viewer (press Q).
#
#   demo/run.sh [--live] [--no-gestures] [--warm]
#
#   --live         Wipe demo/tpch-lake/.visualyze/ instead of loading the checked
#                  model, so an agent builds it live.
#   --no-gestures  Don't start the camera gesture bridge.
#   --warm         Ask Snowflake one question in the background to resume the warehouse.
set -euo pipefail

live=0 gestures=1 warm=0
for arg in "$@"; do
  case "$arg" in
    --live) live=1 ;;
    --no-gestures) gestures=0 ;;
    --warm) warm=1 ;;
    -h|--help) sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $arg" >&2; exit 2 ;;
  esac
done

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
project="$root/demo/tpch-lake"
run_dir="${VISUALYZE_RUN_DIR:-$HOME/Library/Application Support/Visualyze/run}"
socket="$run_dir/viewer.sock"

serving() {
  python3 - "$socket" <<'EOF' 2>/dev/null
import socket, sys
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.settimeout(1)
s.connect(sys.argv[1])
EOF
}

if serving; then
  echo "A Visualyze viewer is already serving ($socket)." >&2
  echo "Close that window first; run.sh starts its own so the Ask box sees this project." >&2
  exit 1
fi

cargo build --locked

if [ "$live" = 1 ]; then
  rm -rf "$project/.visualyze"
  mkdir -p "$project/.visualyze"
else
  mkdir -p "$project/.visualyze"
  cp "$root"/demo/tpch-fallback/*.json "$project/.visualyze/"
fi

export FILAGREE_PROJECT="$project"
export SNOWFLAKE_CONNECTION="${SNOWFLAKE_CONNECTION:-QN29986}"
logs="$project/.visualyze"

pids=()
cleanup() {
  trap - EXIT INT TERM
  for pid in "${pids[@]:-}"; do
    [ -n "$pid" ] || continue
    pkill -TERM -P "$pid" 2>/dev/null || true
    kill -TERM "$pid" 2>/dev/null || true
  done
}
trap cleanup EXIT
trap 'exit 130' INT TERM

(cd "$project" && exec "$root/target/debug/visualyze" --serve --maximized) >"$logs/viewer.log" 2>&1 &
viewer=$!
pids+=("$viewer")

for _ in $(seq 1 80); do
  if [ -S "$socket" ] && [ -f "$run_dir/viewer.token" ] && serving; then
    break
  fi
  if ! kill -0 "$viewer" 2>/dev/null; then
    echo "The viewer exited while starting:" >&2
    tail -8 "$logs/viewer.log" >&2
    exit 1
  fi
  sleep 0.25
done
if ! serving; then
  echo "The viewer did not start serving within 20s (log: $logs/viewer.log)" >&2
  exit 1
fi

if [ "$live" = 0 ]; then
  python3 "$root/demo/preload.py" "$project/.visualyze/model.json" "$project/.visualyze/walkthrough.json"
fi

if [ "$gestures" = 1 ]; then
  (cd "$root/gestures" && exec uv run python -m filagree_gestures) >"$logs/gestures.log" 2>&1 &
  pids+=("$!")
fi

if [ "$warm" = 1 ]; then
  (cd "$project" && exec uv run --with snowflake-connector-python ask/ask.py \
    "Which region earned the most in 1997?") >"$logs/warm.log" 2>&1 &
  pids+=("$!")
fi

cat <<EOF

  Filagree is up.  Press Q in the viewer and ask anything.

  Try: "Explain the codebase and the data it represents"
       "Which region earned the most in 1997?"
       click LINEITEM, then "What's in this table?"
       "Walk me through how a customer orders a part"
       "Why was this order late?"
       "What if PartsHub orders streamed into Snowflake?"

  Logs in demo/tpch-lake/.visualyze/. Close the window or press Ctrl-C to stop.

EOF

status=0
wait "$viewer" || status=$?
exit "$status"
