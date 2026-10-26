"""Client for the Visualyze viewer's per-user socket (visualyze.ipc.v1)."""

import json
import os
import socket
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
            raise ViewerError(error["code"], error["message"])
        return response["result"]

    def close(self) -> None:
        self._reader.close()
        self._sock.close()


class ViewerError(Exception):
    def __init__(self, code: str, message: str) -> None:
        super().__init__(f"{code}: {message}")
        self.code = code
