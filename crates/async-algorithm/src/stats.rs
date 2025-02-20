use core::{fmt::Display, future::Future};

use embassy_futures::select::{select, Either};

use async_kartoffel::{Duration, Instant};

use super::{isqrt, Breakpoint};

pub struct StatsDog {
    sum_duration: Duration,
    counter: u32,
    max_duration: Duration,
    min_duration: Duration,
    sum_sq_duration: u64,
    last_time: Instant,
}
impl Default for StatsDog {
    fn default() -> Self {
        Self::new()
    }
}
impl StatsDog {
    pub fn new() -> Self {
        Self {
            last_time: Instant::now(),
            sum_duration: Default::default(),
            counter: 0,
            max_duration: Default::default(),
            min_duration: Default::default(),
            sum_sq_duration: 0,
        }
    }
    /// benchmark latency for an async function, only valid if there a no other futures running
    pub async fn benchmark<F: Future>(&mut self, f: F) -> F::Output {
        self.restart_timer();
        match select(f, self.feed_continuous()).await {
            Either::First(output) => {
                self.feed();
                output
            }
            Either::Second(_) => unreachable!(),
        }
    }
    /// runs indefinitely, to be used with select
    pub async fn feed_continuous(&mut self) -> ! {
        loop {
            Breakpoint::new().await;
            self.feed();
        }
    }
    pub fn feed(&mut self) {
        let now = Instant::now();
        let duration = (now - self.last_time).unwrap();

        self.sum_duration += duration;
        self.counter += 1;
        self.max_duration = self.max_duration.max(duration);
        self.min_duration = self.min_duration.min(duration);
        self.sum_sq_duration += u64::from(duration.as_ticks()).pow(2);

        self.restart_timer();
    }
    pub fn restart_timer(&mut self) {
        self.last_time = Instant::now();
    }
    pub fn mean(&self) -> u32 {
        self.sum_duration.as_ticks() / self.counter
    }
    pub fn count(&self) -> u32 {
        self.counter
    }
    pub fn total(&self) -> Duration {
        self.sum_duration
    }
    pub fn std(&self) -> u32 {
        // std = 1 / (N - 1) * sum((x - µ)^2)
        //     = 1 / (N - 1) * (sum(x^2) - 2 sum(x) µ + N µ^2)
        //     = 1 / (N - 1) * (sum(x^2) - N µ^2)
        isqrt(
            (self.sum_sq_duration
                - u64::from(self.sum_duration.as_ticks()).pow(2) / u64::from(self.counter))
                / (u64::from(self.counter) - 1),
        ) as u32
    }
}
impl Display for StatsDog {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "latency stats")?;
        writeln!(f, "n    : {}", self.counter)?;
        writeln!(f, "total: {}", self.sum_duration.as_ticks())?;
        writeln!(f, "min  : {}", self.min_duration.as_ticks())?;
        writeln!(f, "mean : {}", self.mean())?;
        writeln!(f, "max  : {}", self.max_duration.as_ticks())?;
        writeln!(f, "std  : {}", self.std())
    }
}
