# Gestures

Hand gestures from the laptop camera drive the Visualyze viewer over its local socket. Uses MediaPipe's built-in gesture recognizer (`models/gesture_recognizer.task`, float16 v1, Apache-2.0), copied from AirCam.

| Gesture | Action |
| --- | --- |
| Open palm facing the camera, turned like a dial | Rotate |
| Fist, moved toward/away from the camera | Zoom in/out |
| One finger up, swiped to your left | Next step |
| One finger up, swiped to your right | Previous step |

Start the viewer with a walkthrough first (see `demo/README.md`), then from this folder:

```sh
uv run python -m filagree_gestures --preview
```

`--preview` shows the camera with the recognized label; press Q or Esc to quit. `--dry-run` prints actions without a viewer. `--camera N` picks another camera. The terminal app you run it from needs camera access (System Settings > Privacy & Security > Camera).

Tune in `filagree_gestures/mapping.py`: `DIAL_GAIN` (flip its sign to reverse rotation), `DIAL_DEADBAND`, `ZOOM_GAIN`, `ZOOM_DEADBAND`, `SWIPE_DISTANCE`, `SWIPE_MS`, `SWIPE_COOLDOWN_MS`, `GRACE_MS` and `MIN_SCORE`.
