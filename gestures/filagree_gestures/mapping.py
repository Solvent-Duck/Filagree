"""Turn per-frame hand observations into viewer actions. Pure: no camera, no socket.

Four actions:
- Open palm, turned like a dial: rotate
- Fist, moved toward/away from the camera: zoom
- One finger (pointing up), swiped to your left: next step
- One finger, swiped to your right: previous step
"""

import math
from collections import deque
from dataclasses import dataclass

from filagree_gestures.one_euro import OneEuroFilter

MIN_SCORE = 0.6
# The classifier drops to None for 1-3 frames mid-gesture; keep the last label
# through gaps this short so rotations and swipes don't restart.
GRACE_MS = 250
# Radians of yaw per radian the palm turns; deadband ignores tracking jitter.
DIAL_GAIN = 1.5
DIAL_DEADBAND = 0.004
# Zoom per unit change in log palm size; deadband ignores tracking jitter.
ZOOM_GAIN = 2.5
ZOOM_DEADBAND = 0.004
# A swipe is this fraction of the frame width within SWIPE_MS.
SWIPE_DISTANCE = 0.15
SWIPE_MS = 400
# After a swipe, ignore all finger motion this long so the hand's return
# doesn't swipe back.
SWIPE_COOLDOWN_MS = 1500


@dataclass(frozen=True)
class Hand:
    timestamp_ms: int
    label: str | None
    score: float
    x: float
    y: float
    size: float
    angle: float = 0.0  # palm roll in radians; 0 is fingers up


@dataclass(frozen=True)
class Action:
    kind: str  # next, previous, orbit, zoom
    yaw: float = 0.0
    zoom: float = 0.0


class Mapper:
    def __init__(self) -> None:
        self._stable: str | None = None
        self._seen_ms = 0
        self._mode: str | None = None
        self._last: float | None = None
        self._unwrapped = 0.0
        self._raw_angle: float | None = None
        self._filter = OneEuroFilter(1.5, 0.3)
        self._track: deque[tuple[int, float]] = deque()
        self._cooldown_until = 0

    def step(self, hand: Hand | None, now: int) -> list[Action]:
        raw = hand.label if hand is not None and hand.score >= MIN_SCORE else None
        if raw is not None:
            self._stable, self._seen_ms = raw, now
        elif now - self._seen_ms > GRACE_MS:
            self._stable = None
        mode = {"Open_Palm": "orbit", "Closed_Fist": "zoom", "Pointing_Up": "swipe"}.get(
            self._stable or ""
        )
        if mode != self._mode:
            self._mode = mode
            self._last = None
            self._filter = OneEuroFilter(1.5, 0.3)
            self._raw_angle = None
            self._track.clear()
        if hand is None or mode is None:
            return []
        if mode == "swipe":
            return self._swipe(hand.x, now)
        return self._continuous(mode, hand)

    def _swipe(self, x: float, now: int) -> list[Action]:
        if now < self._cooldown_until:
            return []
        self._track.append((now, x))
        while now - self._track[0][0] > SWIPE_MS:
            self._track.popleft()
        travel = x - self._track[0][1]
        if abs(travel) < SWIPE_DISTANCE:
            return []
        self._cooldown_until = now + SWIPE_COOLDOWN_MS
        self._track.clear()
        # The camera image is not mirrored: your left is higher x.
        return [Action("next" if travel > 0 else "previous")]

    def _continuous(self, mode: str, hand: Hand) -> list[Action]:
        if mode == "orbit":
            # Unwrap so a turn through straight down doesn't jump a full circle.
            if self._raw_angle is not None:
                turn = hand.angle - self._raw_angle
                self._unwrapped += math.atan2(math.sin(turn), math.cos(turn))
            self._raw_angle = hand.angle
            value, gain, deadband = self._unwrapped, DIAL_GAIN, DIAL_DEADBAND
        else:
            value, gain, deadband = math.log(hand.size), ZOOM_GAIN, ZOOM_DEADBAND
        smoothed = self._filter(value, hand.timestamp_ms)
        last = self._last
        if last is None:
            self._last = smoothed
            return []
        # Hold the reference until motion clears the deadband, so slow turns add up.
        if abs(smoothed - last) < deadband:
            return []
        self._last = smoothed
        if mode == "orbit":
            # Turning your palm clockwise (as you see it) turns the assembly with it.
            return [Action("orbit", yaw=(last - smoothed) * gain)]
        return [Action("zoom", zoom=(smoothed - last) * gain)]
