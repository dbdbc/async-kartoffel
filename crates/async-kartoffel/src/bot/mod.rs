mod arm;
mod compass;
pub mod error;
mod motor;
mod radar;

pub use arm::Arm;
pub use compass::Compass;
pub use motor::Motor;
pub use radar::{Radar, RadarScan, RadarScanWeak};

pub struct Bot {
    pub motor: Motor,
    pub radar: Radar,
    pub arm: Arm,
    pub compass: Compass,
}

impl Bot {
    /// can be taken exactly once
    pub fn take() -> Self {
        #[allow(static_mut_refs)]
        Self {
            motor: unsafe { motor::MOTOR.take() },
            arm: unsafe { arm::ARM.take() },
            compass: unsafe { compass::COMPASS.take() },
            radar: unsafe { radar::RADAR.take() },
        }
    }
}

struct Singleton<T> {
    instance: Option<T>,
}
impl<T> Singleton<T> {
    fn take(&mut self) -> T {
        core::mem::take(&mut self.instance).unwrap()
    }
}
