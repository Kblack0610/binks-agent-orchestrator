"""
Simple rate limiter implementation using sliding window algorithm.
"""

import time
from collections import deque
from threading import Lock


class RateLimiter:
    """
    A thread-safe rate limiter that allows max N requests per minute.

    Uses a sliding window algorithm with a deque to track request timestamps.
    """

    def __init__(self, max_requests: int, window_seconds: float = 60.0):
        """
        Initialize the rate limiter.

        Args:
            max_requests: Maximum number of requests allowed within the window.
            window_seconds: Time window in seconds (default: 60 for per-minute limiting).

        Raises:
            ValueError: If max_requests < 1 or window_seconds <= 0.
        """
        if max_requests < 1:
            raise ValueError("max_requests must be at least 1")
        if window_seconds <= 0:
            raise ValueError("window_seconds must be positive")

        self._max_requests = max_requests
        self._window_seconds = window_seconds
        self._timestamps: deque[float] = deque()
        self._lock = Lock()

    def _cleanup_expired(self, now: float) -> None:
        """Remove timestamps outside the current window."""
        cutoff = now - self._window_seconds
        while self._timestamps and self._timestamps[0] < cutoff:
            self._timestamps.popleft()

    def allow(self) -> bool:
        """
        Check if a request is allowed and record it if so.

        Returns:
            True if the request is allowed, False if rate limited.
        """
        now = time.monotonic()

        with self._lock:
            self._cleanup_expired(now)

            if len(self._timestamps) < self._max_requests:
                self._timestamps.append(now)
                return True
            return False

    def remaining(self) -> int:
        """
        Get the number of remaining requests allowed in the current window.

        Returns:
            Number of requests that can still be made.
        """
        now = time.monotonic()

        with self._lock:
            self._cleanup_expired(now)
            return max(0, self._max_requests - len(self._timestamps))

    def reset_time(self) -> float:
        """
        Get seconds until the oldest request expires (window resets).

        Returns:
            Seconds until at least one more request will be allowed,
            or 0.0 if requests are currently available.
        """
        now = time.monotonic()

        with self._lock:
            self._cleanup_expired(now)

            if len(self._timestamps) < self._max_requests:
                return 0.0

            oldest = self._timestamps[0]
            return max(0.0, (oldest + self._window_seconds) - now)


if __name__ == "__main__":
    # Demo usage
    limiter = RateLimiter(max_requests=5, window_seconds=10)

    print("Testing rate limiter (5 requests per 10 seconds):")
    for i in range(7):
        allowed = limiter.allow()
        remaining = limiter.remaining()
        print(f"  Request {i+1}: {'allowed' if allowed else 'DENIED'} (remaining: {remaining})")

    reset = limiter.reset_time()
    print(f"\nNext request available in: {reset:.2f} seconds")
