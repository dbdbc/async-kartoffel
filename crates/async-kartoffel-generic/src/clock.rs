use core::future::Future;
use core::marker::PhantomData;
use core::ops::{Add, AddAssign, Div, Sub};
use core::pin::Pin;
use core::task::{Context, Poll};
use core::write;

pub trait ClockBackend {
    fn now() -> u32;
    fn ticks_per_milli() -> u32;
}

pub struct Instant<C: ClockBackend> {
    ticks: u32,
    _phantom: PhantomData<C>,
}

impl<C: ClockBackend> core::hash::Hash for Instant<C> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.ticks.hash(state);
    }
}

impl<C: ClockBackend> core::fmt::Debug for Instant<C> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Instant")
            .field("ticks", &self.ticks)
            .finish()
    }
}

impl<C: ClockBackend> Copy for Instant<C> {}

impl<C: ClockBackend> Clone for Instant<C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<C: ClockBackend> Eq for Instant<C> {}

impl<C: ClockBackend> PartialEq for Instant<C> {
    fn eq(&self, other: &Self) -> bool {
        self.ticks == other.ticks
    }
}

impl<C: ClockBackend> PartialOrd for Instant<C> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<C: ClockBackend> Ord for Instant<C> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.ticks.cmp(&other.ticks)
    }
}

impl<C: ClockBackend> core::fmt::Display for Instant<C> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "t={}", self.ticks)
    }
}

impl<C: ClockBackend> Instant<C> {
    pub fn now() -> Self {
        Self {
            ticks: C::now(),
            _phantom: PhantomData,
        }
    }
}

pub struct Duration<C: ClockBackend> {
    ticks: u32,
    _phantom: PhantomData<C>,
}

impl<C: ClockBackend> core::hash::Hash for Duration<C> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.ticks.hash(state);
    }
}

impl<C: ClockBackend> core::fmt::Debug for Duration<C> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Instant")
            .field("ticks", &self.ticks)
            .finish()
    }
}

impl<C: ClockBackend> Default for Duration<C> {
    fn default() -> Self {
        Self {
            ticks: Default::default(),
            _phantom: PhantomData,
        }
    }
}

impl<C: ClockBackend> Copy for Duration<C> {}

impl<C: ClockBackend> Clone for Duration<C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<C: ClockBackend> Eq for Duration<C> {}

impl<C: ClockBackend> PartialEq for Duration<C> {
    fn eq(&self, other: &Self) -> bool {
        self.ticks == other.ticks
    }
}

impl<C: ClockBackend> PartialOrd for Duration<C> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<C: ClockBackend> Ord for Duration<C> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.ticks.cmp(&other.ticks)
    }
}

impl<C: ClockBackend> Duration<C> {
    pub fn since(instant: Instant<C>) -> Option<Self> {
        Instant::now() - instant
    }

    pub fn from_ticks(n: u32) -> Self {
        Self {
            ticks: n,
            _phantom: PhantomData,
        }
    }

    pub fn from_millis(n: u32) -> Self {
        Self::from_ticks(n * C::ticks_per_milli())
    }

    pub fn from_secs(n: u32) -> Self {
        Self::from_ticks(n * 1_000 * C::ticks_per_milli())
    }

    pub fn as_ticks(&self) -> u32 {
        self.ticks
    }

    pub fn as_secs_floor(&self) -> u32 {
        self.ticks.div(1_000 * C::ticks_per_milli())
    }

    pub fn as_secs_ceil(&self) -> u32 {
        self.ticks.div_ceil(1_000 * C::ticks_per_milli())
    }

    pub fn as_millis_ceil(&self) -> u32 {
        self.ticks.div_ceil(C::ticks_per_milli())
    }

    pub fn as_millis_floor(&self) -> u32 {
        self.ticks.div(C::ticks_per_milli())
    }
}

impl<C: ClockBackend> core::fmt::Display for Duration<C> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Δt={}", self.ticks)
    }
}

impl<C: ClockBackend> Add<Duration<C>> for Instant<C> {
    type Output = Instant<C>;

    fn add(self, rhs: Duration<C>) -> Self::Output {
        Instant {
            ticks: self.ticks + rhs.ticks,
            _phantom: PhantomData,
        }
    }
}

impl<C: ClockBackend> AddAssign<Duration<C>> for Instant<C> {
    fn add_assign(&mut self, rhs: Duration<C>) {
        *self = *self + rhs;
    }
}

impl<C: ClockBackend> Sub for Instant<C> {
    type Output = Option<Duration<C>>;

    fn sub(self, rhs: Self) -> Self::Output {
        if self.ticks >= rhs.ticks {
            Some(Duration {
                ticks: self.ticks - rhs.ticks,
                _phantom: PhantomData,
            })
        } else {
            None
        }
    }
}

impl<C: ClockBackend> Add for Duration<C> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            ticks: self.ticks + rhs.ticks,
            _phantom: PhantomData,
        }
    }
}

impl<C: ClockBackend> Sub for Duration<C> {
    type Output = Option<Self>;

    fn sub(self, rhs: Self) -> Self::Output {
        if self.ticks >= rhs.ticks {
            Some(Self {
                ticks: self.ticks - rhs.ticks,
                _phantom: PhantomData,
            })
        } else {
            None
        }
    }
}

impl<C: ClockBackend> AddAssign for Duration<C> {
    fn add_assign(&mut self, rhs: Self) {
        self.ticks += rhs.ticks;
    }
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
// stolen from embassy
pub struct Timer<C: ClockBackend> {
    expires_at: Instant<C>,
    yielded_once: bool,
}

impl<C: ClockBackend> Timer<C> {
    pub fn at(expires_at: Instant<C>) -> Self {
        Self {
            expires_at,
            yielded_once: false,
        }
    }

    pub fn after(duration: Duration<C>) -> Self {
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

impl<C: ClockBackend> Unpin for Timer<C> {}

impl<C: ClockBackend> Future for Timer<C> {
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
