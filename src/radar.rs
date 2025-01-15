use critical_section::Mutex;

use crate::{rdi, wri, Error, Singleton, MEM_RADAR};
use core::{
    cell::Cell,
    future::poll_fn,
    marker::PhantomData,
    num::NonZeroU64,
    task::{Poll, Waker},
};

#[derive(Clone, Default)]
struct Guard {
    active_uuid: u32,
    n_scans: u32,
    waker: Option<Waker>,
}

static GUARD: Mutex<Cell<Guard>> = Mutex::new(Cell::new(Guard {
    active_uuid: 0,
    n_scans: 0,
    waker: None,
}));

impl Guard {
    #[inline(always)]
    fn is_radar_ready() -> bool {
        rdi(MEM_RADAR, 0) == 1
    }

    fn with_critical_section<T>(f: impl FnOnce(&mut Self) -> T) -> T {
        critical_section::with(|cs| {
            let cell = GUARD.borrow(cs);
            let mut guard = cell.take();
            let t = f(&mut guard);
            cell.set(guard);
            t
        })
    }

    async fn wait_unlocked() {
        poll_fn(|cx| {
            Self::with_critical_section(|guard| {
                if guard.n_scans == 0 {
                    Poll::Ready(())
                } else {
                    guard.waker = Some(cx.waker().clone());
                    Poll::Pending
                }
            })
        })
        .await
    }

    // caller has to verify size
    fn generate<Size: RadarSize>(uuid: u32) -> Option<RadarScan<Size>> {
        Self::with_critical_section(|guard| {
            if uuid == guard.active_uuid {
                guard.n_scans += 1;
                Some(RadarScan(PhantomData))
            } else {
                None
            }
        })
    }
    fn increase_count() {
        Self::with_critical_section(|guard| {
            guard.n_scans += 1;
        })
    }
    fn decrease_count() {
        Self::with_critical_section(|guard| {
            assert!(guard.n_scans > 0);
            guard.n_scans -= 1;
            if guard.n_scans == 0 {
                if let Some(waker) = guard.waker.take() {
                    waker.wake()
                }
            }
        })
    }
    fn try_execute_scan<Size: RadarSize>() -> Result<(), Error> {
        Self::with_critical_section(|guard| {
            if guard.n_scans == 0 {
                if Self::is_radar_ready() {
                    wri(MEM_RADAR, 0, Size::D as u32);

                    // can fail after u32::MAX iterations
                    guard.active_uuid = guard.active_uuid.wrapping_add(1);
                    Ok(())
                } else {
                    Err(Error::NotReady)
                }
            } else {
                Err(Error::Blocked)
            }
        })
    }
    fn get_active_uuid() -> u32 {
        Self::with_critical_section(|guard| guard.active_uuid)
    }
}

// fn generate_scan() ->

mod private {
    pub trait Sealed {}
}
pub trait RadarSize: private::Sealed {
    const R: u8;
    const D: u8 = 2 * Self::R + 1;
}
pub enum D3 {}
impl private::Sealed for D3 {}
impl RadarSize for D3 {
    const R: u8 = 1;
}
pub enum D5 {}
impl private::Sealed for D5 {}
impl RadarSize for D5 {
    const R: u8 = 2;
}
pub enum D7 {}
impl private::Sealed for D7 {}
impl RadarSize for D7 {
    const R: u8 = 3;
}
pub enum D9 {}
impl private::Sealed for D9 {}
impl RadarSize for D9 {
    const R: u8 = 4;
}

#[non_exhaustive]
pub struct RadarScan<Size: RadarSize>(PhantomData<Size>);

impl<Size: RadarSize> RadarScan<Size> {
    // TODO
    // vec instead of indices
    // tiles instead of chars
    // iterator over scanned tiles and positions
    // bots positions excluding self

    #[inline(always)]
    pub fn at(&self, dx: i8, dy: i8) -> char {
        self.get_ex(dx, dy, 0) as u8 as char
    }

    #[inline(always)]
    pub fn bot_at(&self, dx: i8, dy: i8) -> Option<NonZeroU64> {
        let d1 = self.get_d1(dx, dy) as u64;
        let d2 = self.get_d2(dx, dy) as u64;

        NonZeroU64::new((d1 << 32) | d2)
    }

    fn get_d1(&self, dx: i8, dy: i8) -> u32 {
        self.get_ex(dx, dy, 1)
    }

    fn get_d2(&self, dx: i8, dy: i8) -> u32 {
        self.get_ex(dx, dy, 2)
    }

    fn get_ex(&self, dx: i8, dy: i8, z: u8) -> u32 {
        let x = (dx + Size::R as i8) as usize;
        let y = (dy + Size::R as i8) as usize;
        let z = z as usize;

        rdi(
            MEM_RADAR,
            1 + z * (Size::D * Size::D) as usize + y * (Size::D as usize) + x,
        )
    }

    pub fn weak(&self) -> RadarScanWeak<Size> {
        RadarScanWeak {
            uuid: Guard::get_active_uuid(),
            _phantom: PhantomData,
        }
    }
}

impl<Size: RadarSize> Clone for RadarScan<Size> {
    fn clone(&self) -> Self {
        Guard::increase_count();
        Self(PhantomData)
    }
}

impl<Size: RadarSize> Drop for RadarScan<Size> {
    fn drop(&mut self) {
        Guard::decrease_count();
    }
}

/// RadarScan that does not block new scan, but can't be used directly. Instead, you can try to
/// upgrade it to a full scan
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RadarScanWeak<Size: RadarSize> {
    uuid: u32,
    _phantom: PhantomData<Size>,
}

impl<Size: RadarSize> RadarScanWeak<Size> {
    pub fn upgrade(&self) -> Option<RadarScan<Size>> {
        Guard::generate(self.uuid)
    }
}

#[non_exhaustive]
pub struct Radar;

pub static mut RADAR: Singleton<Radar> = Singleton {
    instance: Some(Radar),
};

impl Radar {
    pub fn is_ready(&self) -> bool {
        Guard::is_radar_ready()
    }
    pub async fn wait(&self) {
        poll_fn(|cx| {
            if self.is_ready() {
                Poll::Ready(())
            } else {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        })
        .await;
    }
    pub fn try_scan<Size: RadarSize>(&mut self) -> Result<RadarScan<Size>, Error> {
        let res = Guard::try_execute_scan::<Size>();
        match res {
            Ok(()) => Ok(RadarScan(PhantomData)),
            Err(err) => Err(err),
        }
    }
    pub async fn scan<Size: RadarSize>(&mut self) -> RadarScan<Size> {
        self.wait().await;
        Guard::wait_unlocked().await;
        Guard::try_execute_scan::<Size>().unwrap();
        RadarScan(PhantomData)
    }
}
