"""Publish a model and present a walkthrough to a running `visualyze --serve`.

    python3 demo/preload.py MODEL.json [WALKTHROUGH.json]

Stdlib only. Speaks visualyze.ipc.v1 (same as gestures/filagree_gestures/viewer.py):
newline-delimited JSON over the per-user socket, starting with hello.
"""

import json
import os
import socket
import sys
from pathlib import Path

PROTOCOL = "visualyze.ipc.v1"


def run_dir() -> Path:
    if env := os.environ.get("VISUALYZE_RUN_DIR"):
        return Path(env)
    return Path.home() / "Library/Application Support/Visualyze/run"


class Viewer:
    def __init__(self) -> None:
        directory = run_dir()
        token = (directory / "viewer.token").read_text().strip()
        self._sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self._sock.settimeout(60)
        self._sock.connect(str(directory / "viewer.sock"))
        self._reader = self._sock.makefile("rb")
        self._id = 0
        self.call("hello", {"token": token, "protocol": PROTOCOL})

    def call(self, method: str, params: dict) -> dict:
        self._id += 1
        line = json.dumps({"id": self._id, "method": method, "params": params}) + "\n"
        self._sock.sendall(line.encode())
        response = json.loads(self._reader.readline())
        if error := response.get("error"):
            raise SystemExit(f"{method} failed: {error['code']}: {error['message']}")
        return response["result"]


def main() -> None:
    if len(sys.argv) not in (2, 3):
        sys.exit(__doc__)
    model = Path(sys.argv[1]).resolve()
    viewer = Viewer()
    # publish takes the model as a JSON string; path lets F5 reload it.
    viewer.call("publish", {"content": model.read_text(), "path": str(model)})
    print(f"Published {model.name}")
    if len(sys.argv) == 3:
        walkthrough = json.loads(Path(sys.argv[2]).read_text())
        viewer.call("present", {"walkthrough": walkthrough, "autoplay": False})
        print(f"Presented {Path(sys.argv[2]).name}")


if __name__ == "__main__":
    main()
