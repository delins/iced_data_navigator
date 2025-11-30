use std::time::{Duration, Instant};

/// A timer that sets a target time in the future and can check whether that time has arrived.
#[derive(Debug)]
pub struct Timer {
    target: Instant,
    interval: u64,
}

impl Timer {
    /// Creates a new timer with timer `now` + `interval`.
    pub fn new(now: Instant, interval: u64) -> Self {
        Timer {
            target: now + Duration::from_millis(interval),
            interval
        }
    }

    /// The set target time.
    pub fn target(&self) -> Instant {
        self.target
    }

    /// Tests whether the target has been reached. Also checks how long overdue we are, in ms.
    pub fn test(&self, now: &Instant) -> (bool, u64) {
        if *now > self.target {
            let overdue = (*now - self.target).as_millis() as u64;
            (true, overdue)
        } else {
            (false, 0)
        }
    }

    /// Sets the target time at now + the `interval` set at creation time.
    pub fn set_at_interval(&mut self, now: &Instant) -> Instant {
        self.target = *now + Duration::from_millis(self.interval);
        self.target
    }
}

