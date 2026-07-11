use std::{cell::Cell, time::SystemTime};

pub trait Clock {
    fn now(&self) -> SystemTime;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

#[derive(Debug)]
pub struct FakeClock {
    current: Cell<SystemTime>,
}

impl FakeClock {
    pub const fn new(current: SystemTime) -> Self {
        Self {
            current: Cell::new(current),
        }
    }

    pub fn set(&self, current: SystemTime) {
        self.current.set(current);
    }
}

impl Clock for FakeClock {
    fn now(&self) -> SystemTime {
        self.current.get()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn fake_clock_is_deterministic_and_settable() {
        let first = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let second = first + Duration::from_secs(90);
        let clock = FakeClock::new(first);

        assert_eq!(clock.now(), first);
        assert_eq!(clock.now(), first);
        clock.set(second);
        assert_eq!(clock.now(), second);
    }

    #[test]
    fn system_clock_reads_current_time_without_a_runtime() {
        let before = SystemTime::now();
        let observed = SystemClock.now();
        let after = SystemTime::now();

        assert!(observed >= before);
        assert!(observed <= after);
    }
}
