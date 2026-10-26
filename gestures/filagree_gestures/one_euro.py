"""Scalar One Euro filter using recorded timestamps, never wall-clock time."""

import math


def _alpha(cutoff: float, dt: float) -> float:
    r = 2 * math.pi * cutoff * dt
    return r / (r + 1)


class OneEuroFilter:
    def __init__(
        self, min_cutoff: float = 1.0, beta: float = 0.007, derivative_cutoff: float = 1.0
    ) -> None:
        if not all(math.isfinite(v) for v in (min_cutoff, beta, derivative_cutoff)):
            raise ValueError("Filter parameters must be finite")
        if min_cutoff <= 0 or beta < 0 or derivative_cutoff <= 0:
            raise ValueError("Cutoffs must be positive and beta nonnegative")
        self.min_cutoff = min_cutoff
        self.beta = beta
        self.derivative_cutoff = derivative_cutoff
        self._previous: float | None = None
        self._filtered = 0.0
        self._derivative = 0.0
        self._timestamp_ms: int | None = None

    def __call__(self, value: float, timestamp_ms: int) -> float:
        if not math.isfinite(value) or timestamp_ms < 0:
            raise ValueError("Filter requires a finite value and nonnegative timestamp")
        if self._timestamp_ms is not None and timestamp_ms <= self._timestamp_ms:
            raise ValueError("Filter timestamps must strictly increase")
        if self._previous is None:
            self._filtered = value
        else:
            assert self._timestamp_ms is not None
            dt = (timestamp_ms - self._timestamp_ms) / 1000
            derivative = (value - self._previous) / dt
            a_d = _alpha(self.derivative_cutoff, dt)
            self._derivative += a_d * (derivative - self._derivative)
            cutoff = self.min_cutoff + self.beta * abs(self._derivative)
            self._filtered += _alpha(cutoff, dt) * (value - self._filtered)
        self._previous = value
        self._timestamp_ms = timestamp_ms
        return self._filtered
