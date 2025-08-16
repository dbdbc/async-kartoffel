use core::{fmt::Display, future::Future};

use embassy_futures::select::{Either, select};

use async_kartoffel_generic::{ClockBackend, Duration, Instant};

use super::Breakpoint;

/// A watchdog-like utility that can e.g. be used to track how often a certain Future is polled.
///
/// ```no_run
/// use async_kartoffel_generic::{ClockBackend, Duration};
/// use async_algorithm::StatsDog;
/// async fn with_watchdog<C: ClockBackend>() {
///     let mut dog = StatsDog::<C>::new();
///     let mut i = 0;
///     loop {
///         let elapsed = dog.feed();
///         if elapsed > Duration::<C>::from_ticks(20_000) {
///             println!("warning: blocked {}", elapsed);
///         }
///         i += 1;
///         if i > 10 {
///             println!("{}", dog);
///             break;
///         }
///     }
/// }
/// ```
pub struct StatsDog<C: ClockBackend> {
    sum_duration: Duration<C>,
    counter: u32,
    max_duration: Duration<C>,
    min_duration: Duration<C>,
    sum_sq_duration: u64,
    last_time: Instant<C>,
}
impl<C: ClockBackend> Default for StatsDog<C> {
    fn default() -> Self {
        Self::new()
    }
}
impl<C: ClockBackend> StatsDog<C> {
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
    /// resets the timer, and adds the elapsed time to the gathered statistics
    pub fn feed(&mut self) -> Duration<C> {
        let now = Instant::now();
        let duration = (now - self.last_time).unwrap();

        self.sum_duration += duration;
        self.counter += 1;
        self.max_duration = self.max_duration.max(duration);
        self.min_duration = self.min_duration.min(duration);
        self.sum_sq_duration += u64::from(duration.as_ticks()).pow(2);

        self.restart_timer();

        duration
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
    pub fn total(&self) -> Duration<C> {
        self.sum_duration
    }
    /// empirical standard deviation
    pub fn std(&self) -> u32 {
        // std = 1 / (N - 1) * sum((x - µ)^2)
        //     = 1 / (N - 1) * (sum(x^2) - 2 sum(x) µ + N µ^2)
        //     = 1 / (N - 1) * (sum(x^2) - N µ^2)
        ((self.sum_sq_duration
            - u64::from(self.sum_duration.as_ticks()).pow(2) / u64::from(self.counter))
            / (u64::from(self.counter) - 1))
            .isqrt() as u32
    }
}
impl<C: ClockBackend> Display for StatsDog<C> {
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
