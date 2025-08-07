use core::future::Future;
use core::ops::{Add, AddAssign, Div, Sub};
use core::pin::Pin;
use core::task::{Context, Poll};
use core::write;

use kartoffel::timer_ticks;

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Debug, Hash)]
pub struct Instant(u32);

impl core::fmt::Display for Instant {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "t={}", self.0)
    }
}

impl Instant {
    pub fn now() -> Self {
        Self(timer_ticks())
    }
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Debug, Default, Hash)]
pub struct Duration(u32);

impl Duration {
    pub fn from_ticks(n: u32) -> Self {
        Self(n)
    }

    pub fn from_millis(n: u32) -> Self {
        Self(n * 64)
    }

    pub fn from_secs(n: u32) -> Self {
        Self(n * 64_000)
    }

    pub fn as_ticks(&self) -> u32 {
        self.0
    }

    pub fn as_secs_floor(&self) -> u32 {
        self.0.div(64_000)
    }

    pub fn as_secs_ceil(&self) -> u32 {
        self.0.div_ceil(64_000)
    }

    pub fn as_millis_ceil(&self) -> u32 {
        self.0.div_ceil(64)
    }

    pub fn as_millis_floor(&self) -> u32 {
        self.0.div(64)
    }
}

impl core::fmt::Display for Duration {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Δt={}", self.0)
    }
}

impl Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, rhs: Duration) -> Self::Output {
        Instant(self.0 + rhs.0)
    }
}

impl AddAssign<Duration> for Instant {
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs;
    }
}

impl Sub for Instant {
    type Output = Option<Duration>;

    fn sub(self, rhs: Self) -> Self::Output {
        if self.0 >= rhs.0 {
            Some(Duration(self.0 - rhs.0))
        } else {
            None
        }
    }
}

impl Add for Duration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Duration {
    type Output = Option<Self>;

    fn sub(self, rhs: Self) -> Self::Output {
        if self.0 >= rhs.0 {
            Some(Self(self.0 - rhs.0))
        } else {
            None
        }
    }
}

impl AddAssign for Duration {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

#[derive(Clone, Copy)]
pub struct Cooldown {
    pub time_started: Instant,
    pub cooldown_type: CooldownType,
}

impl Cooldown {
    /// start the cooldown
    pub fn start_new(cooldown_type: CooldownType) -> Self {
        Self {
            time_started: Instant::now(),
            cooldown_type,
        }
    }
    pub fn expected_done(&self) -> Instant {
        self.time_started + self.cooldown_type.expected_duration()
    }
}

#[derive(Clone, Copy)]
pub enum CooldownType {
    Stab,
    Pick,
    Forward,
    Turn,
    Radar3,
    Radar5,
    Radar7,
    Radar9,
    Compass,
}

impl CooldownType {
    pub fn expected_duration(&self) -> Duration {
        match self {
            CooldownType::Stab => Duration(60_000),
            CooldownType::Pick => Duration(60_000),
            CooldownType::Forward => Duration(20_000),
            CooldownType::Turn => Duration(10_000),
            CooldownType::Radar3 => Duration(10_000),
            CooldownType::Radar5 => Duration(15_000),
            CooldownType::Radar7 => Duration(22_000),
            CooldownType::Radar9 => Duration(30_000),
            CooldownType::Compass => Duration(128_000),
        }
    }
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
// stolen from embassy
pub struct Timer {
    expires_at: Instant,
    yielded_once: bool,
}

impl Timer {
    pub fn at(expires_at: Instant) -> Self {
        Self {
            expires_at,
            yielded_once: false,
        }
    }

    pub fn after(duration: Duration) -> Self {
        Self {
            expires_at: Instant::now() + duration,
            yielded_once: false,
        }
    }

    #[inline]
    pub fn after_ticks(ticks: u32) -> Self {
        Self::after(Duration::from_ticks(ticks))
    }

    #[inline]
    pub fn after_millis(millis: u32) -> Self {
        Self::after(Duration::from_millis(millis))
    }

    #[inline]
    pub fn after_secs(secs: u32) -> Self {
        Self::after(Duration::from_secs(secs))
    }

    pub fn wait_blocking(self) {
        while Instant::now() < self.expires_at {
            //
        }
    }
}

impl Unpin for Timer {}

impl Future for Timer {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.yielded_once && self.expires_at <= Instant::now() {
            Poll::Ready(())
        } else {
            // TODO currently instantly schedule another poll, schedule wake further into future
            cx.waker().wake_by_ref();
            self.yielded_once = true;
            Poll::Pending
        }
    }
}
