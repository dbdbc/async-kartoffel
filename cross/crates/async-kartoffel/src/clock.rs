use async_kartoffel_generic::ClockBackend;
use kartoffel::timer_ticks;

pub enum KartoffelClock {}
impl ClockBackend for KartoffelClock {
    fn now() -> u32 {
        timer_ticks()
    }

    fn ticks_per_milli() -> u32 {
        64
    }
}

pub type Timer = async_kartoffel_generic::Timer<KartoffelClock>;
pub type Instant = async_kartoffel_generic::Instant<KartoffelClock>;
pub type Duration = async_kartoffel_generic::Duration<KartoffelClock>;
