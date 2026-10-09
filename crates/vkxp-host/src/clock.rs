//! Where a session reads the time from.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// A monotonic clock. Only differences between its readings mean anything.
pub trait Clock {
    /// The time that has passed since the clock's origin.
    fn now(&self) -> Duration;
}

/// The real clock, counting from its creation.
#[derive(Debug, Clone, Copy)]
pub struct SystemClock {
    origin: Instant,
}

impl SystemClock {
    pub fn new() -> SystemClock {
        SystemClock {
            origin: Instant::now(),
        }
    }

    /// The instant at which this clock reads `time`.
    pub fn instant_at(&self, time: Duration) -> Instant {
        self.origin + time
    }
}

impl Default for SystemClock {
    fn default() -> SystemClock {
        SystemClock::new()
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }
}

/// A clock that only moves when it is told to: the fake clock of the tests and the virtual
/// clock of a headless run. Clones share one time.
#[derive(Debug, Clone, Default)]
pub struct ManualClock {
    time: Arc<Mutex<Duration>>,
}

impl ManualClock {
    /// A clock that reads zero.
    pub fn new() -> ManualClock {
        ManualClock::default()
    }

    /// Moves the clock forward by `step`.
    pub fn advance(&self, step: Duration) {
        *self.lock() += step;
    }

    /// Moves the clock forward to `time`. A time in the past leaves it where it is: the clock
    /// never runs backwards.
    pub fn advance_to(&self, time: Duration) {
        let mut now = self.lock();
        *now = (*now).max(time);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Duration> {
        self.time.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Duration {
        *self.lock()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_clock_moves_only_when_told_and_never_backwards() {
        let clock = ManualClock::new();
        let shared = clock.clone();
        assert_eq!(clock.now(), Duration::ZERO);
        clock.advance(Duration::from_millis(25));
        shared.advance_to(Duration::from_millis(40));
        assert_eq!(clock.now(), Duration::from_millis(40));
        clock.advance_to(Duration::from_millis(10));
        assert_eq!(shared.now(), Duration::from_millis(40));
    }

    #[test]
    fn system_clock_is_monotonic_and_maps_back_to_instants() {
        let clock = SystemClock::new();
        let first = clock.now();
        let second = clock.now();
        assert!(second >= first);
        assert!(clock.instant_at(second) >= clock.instant_at(first));
    }
}
