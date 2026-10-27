"""Camera -> MediaPipe gestures -> viewer actions. Run: uv run python -m filagree_gestures"""

import argparse
import math
import sys
import time
from pathlib import Path

import cv2
import mediapipe as mp

from filagree_gestures.mapping import Action, Hand, Mapper
from filagree_gestures.viewer import Viewer, ViewerError

MODEL = Path(__file__).resolve().parent.parent / "models" / "gesture_recognizer.task"


def recognizer():
    options = mp.tasks.vision.GestureRecognizerOptions(
        base_options=mp.tasks.BaseOptions(
            model_asset_path=str(MODEL),
            # MediaPipe 1.0.1 on macOS needs the Metal delegate; CPU aborts natively.
            delegate=mp.tasks.BaseOptions.Delegate.GPU
            if sys.platform == "darwin"
            else mp.tasks.BaseOptions.Delegate.CPU,
        ),
        running_mode=mp.tasks.vision.RunningMode.VIDEO,
        num_hands=1,
    )
    return mp.tasks.vision.GestureRecognizer.create_from_options(options)


def hand_from(result, timestamp_ms: int, aspect: float) -> Hand | None:
    if not result.hand_landmarks:
        return None
    marks = result.hand_landmarks[0]
    palm = [marks[i] for i in (0, 5, 9, 13, 17)]
    size = math.hypot(marks[0].x - marks[9].x, marks[0].y - marks[9].y)
    # Wrist to middle knuckle, in square pixels: 0 is fingers up, positive leans
    # toward image right.
    angle = math.atan2((marks[9].x - marks[0].x) * aspect, marks[0].y - marks[9].y)
    top = max(result.gestures[0], key=lambda c: c.score) if result.gestures else None
    label = top.category_name if top and top.category_name != "None" else None
    return Hand(
        timestamp_ms=timestamp_ms,
        label=label,
        score=top.score if top else 0.0,
        x=sum(p.x for p in palm) / 5,
        y=sum(p.y for p in palm) / 5,
        size=max(size, 1e-4),
        angle=angle,
    )


def open_camera(index: int):
    """Retry with fresh handles while a first-use macOS permission prompt settles."""
    deadline = time.monotonic() + 15
    while True:
        capture = cv2.VideoCapture(index, cv2.CAP_AVFOUNDATION)
        if capture.isOpened():
            return capture
        capture.release()
        if time.monotonic() > deadline:
            sys.exit("Camera unavailable: check System Settings > Privacy & Security > Camera")
        print("Waiting for camera access; click Allow if macOS asks.", flush=True)
        time.sleep(1)


def send(viewer: Viewer | None, action: Action) -> None:
    if viewer is None:
        return
    try:
        if action.kind in ("next", "previous"):
            viewer.call("control", {"action": action.kind})
        elif action.kind == "orbit":
            viewer.call("camera", {"orbit": [action.yaw, 0.0]})
        elif action.kind == "zoom":
            viewer.call("camera", {"zoom": action.zoom})
    except ViewerError as error:
        # e.g. "already at the last step" or no walkthrough playing.
        print(f"viewer: {error}", flush=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--camera", type=int, default=0, help="OpenCV camera index")
    parser.add_argument("--preview", action="store_true", help="show the camera and labels")
    parser.add_argument("--dry-run", action="store_true", help="print actions, no viewer")
    args = parser.parse_args()

    viewer = None if args.dry_run else Viewer()
    capture = open_camera(args.camera)
    mapper = Mapper()
    last_ms = -1
    failed_since: float | None = None
    # Trial log: each label change, with the spin/zoom summed over the segment.
    segment = {"label": "", "since": 0, "yaw": 0.0, "zoom": 0.0, "spun": 0.0, "zoomed": 0.0,
               "frames": 0}
    print("Gestures on. Open palm, turn like a dial: rotate. Fist toward/away: zoom.")
    print("One finger swipe to your left: next step; to your right: back. Ctrl-C to stop.",
          flush=True)
    with recognizer() as model:
        try:
            while True:
                ok, frame = capture.read()
                if not ok:
                    # macOS cameras can return empty frames for a moment after
                    # opening, or while another app releases the device.
                    failed_since = failed_since or time.monotonic()
                    if time.monotonic() - failed_since > 5:
                        sys.exit("Camera stopped delivering frames")
                    time.sleep(0.05)
                    continue
                failed_since = None
                now = max(time.monotonic_ns() // 1_000_000, last_ms + 1)
                last_ms = now
                rgba = cv2.cvtColor(frame, cv2.COLOR_BGR2RGBA)
                image = mp.Image(image_format=mp.ImageFormat.SRGBA, data=rgba)
                aspect = frame.shape[1] / frame.shape[0]
                hand = hand_from(model.recognize_for_video(image, now), now, aspect)
                label = "no hand" if hand is None else f"{hand.label}"
                if label != segment["label"]:
                    if segment["frames"]:
                        # Net and total travel: a back-and-forth sweep nets near zero.
                        print(f"  {segment['label']:12} {now - segment['since']:5d} ms  "
                              f"{segment['frames']:3d} frames  "
                              f"yaw {segment['yaw']:+.2f} (travel {segment['spun']:.2f})  "
                              f"zoom {segment['zoom']:+.2f} (travel {segment['zoomed']:.2f})",
                              flush=True)
                    segment.update(label=label, since=now, yaw=0.0, zoom=0.0, spun=0.0,
                                   zoomed=0.0, frames=0)
                segment["frames"] += 1
                for action in mapper.step(hand, now):
                    segment["yaw"] += action.yaw
                    segment["zoom"] += action.zoom
                    segment["spun"] += abs(action.yaw)
                    segment["zoomed"] += abs(action.zoom)
                    if action.kind in ("next", "previous"):
                        print(f"-> {action.kind}", flush=True)
                    send(viewer, action)
                if args.preview:
                    text = f"{hand.label} {hand.score:.2f}" if hand else "no hand"
                    shown = cv2.flip(frame, 1)
                    cv2.putText(shown, text, (20, 40), cv2.FONT_HERSHEY_SIMPLEX, 1.1,
                                (0, 255, 0), 2)
                    cv2.imshow("Filagree gestures", shown)
                    if cv2.waitKey(1) & 0xFF in (ord("q"), 27):
                        break
        except KeyboardInterrupt:
            pass
        finally:
            capture.release()
            if viewer:
                viewer.close()


if __name__ == "__main__":
    main()
