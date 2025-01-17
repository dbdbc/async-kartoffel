use critical_section::Mutex;

use crate::{Distance, Error, Local, Tile};
use core::{
    cell::Cell,
    future::poll_fn,
    marker::PhantomData,
    num::NonZeroU64,
    task::{Poll, Waker},
};

use crate::mem::{radar_get_ex, radar_is_ready, radar_scan};

use super::Singleton;

#[non_exhaustive]
pub struct Radar;

pub(super) static mut RADAR: Singleton<Radar> = Singleton {
    instance: Some(Radar),
};

impl Radar {
    pub fn is_ready(&self) -> bool {
        radar_is_ready()
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
            Ok(()) => Ok(Guard::create_active::<Size>()),
            Err(err) => Err(err),
        }
    }
    pub async fn scan<Size: RadarSize>(&mut self) -> RadarScan<Size> {
        self.wait().await;
        Guard::wait_unlocked().await;
        Guard::try_execute_scan::<Size>().unwrap();
        Guard::create_active::<Size>()
    }
}

#[derive(Clone, Default, Debug)]
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

    // caller has to ensure size is correct
    fn try_create<Size: RadarSize>(uuid: u32) -> Option<RadarScan<Size>> {
        Self::with_critical_section(|guard| {
            if uuid == guard.active_uuid {
                guard.n_scans += 1;
                Some(RadarScan(PhantomData))
            } else {
                None
            }
        })
    }
    // caller has to ensure size is correct
    fn create_active<Size: RadarSize>() -> RadarScan<Size> {
        Self::with_critical_section(|guard| {
            guard.n_scans += 1;
            RadarScan(PhantomData)
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
                if radar_is_ready() {
                    radar_scan(Size::diameter() as u32);

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

mod private {
    pub trait Sealed {}
}
pub trait RadarSize:
    private::Sealed + Clone + Copy + PartialEq + Eq + PartialOrd + Ord + 'static
{
    const R: u8;
    fn diameter() -> u8 {
        Self::R * 2 + 1
    }
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum D3 {}
impl private::Sealed for D3 {}
impl RadarSize for D3 {
    const R: u8 = 1;
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum D5 {}
impl private::Sealed for D5 {}
impl RadarSize for D5 {
    const R: u8 = 2;
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum D7 {}
impl private::Sealed for D7 {}
impl RadarSize for D7 {
    const R: u8 = 3;
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum D9 {}
impl private::Sealed for D9 {}
impl RadarSize for D9 {
    const R: u8 = 4;
}

#[non_exhaustive]
pub struct RadarScan<Size: RadarSize>(PhantomData<Size>);

impl<Size: RadarSize> RadarScan<Size> {
    #[inline(always)]
    fn radar_indices(dist: Distance<Local>) -> Option<(i8, i8)> {
        let (dx, dy) = (dist.right(), dist.back());
        (dx.unsigned_abs() <= Size::R.into() && dy.unsigned_abs() <= Size::R.into())
            .then_some((dx as i8, dy as i8))
    }
    #[inline(always)]
    fn to_distance(dx: i8, dy: i8) -> Distance<Local> {
        Distance::new_local(dx.into(), (-dy).into())
    }

    pub fn at(&self, dist: Distance<Local>) -> Option<Tile> {
        if let Some((dx, dy)) = Self::radar_indices(dist) {
            // unwrap: unknown tile means error
            Some(Tile::from_char(self.at_unchecked(dx, dy)).unwrap())
        } else {
            None
        }
    }

    #[inline(always)]
    fn at_unchecked(&self, dx: i8, dy: i8) -> char {
        radar_get_ex(Size::R, dx, dy, 0) as u8 as char
    }

    pub fn bot_at(&self, dist: Distance<Local>) -> Option<NonZeroU64> {
        if let Some((dx, dy)) = Self::radar_indices(dist) {
            let d1 = radar_get_ex(Size::R, dx, dy, 1) as u64;
            let d2 = radar_get_ex(Size::R, dx, dy, 2) as u64;
            NonZeroU64::new((d1 << 32) | d2)
        } else {
            None
        }
    }

    /// Scanned tiles matching tile excluding (0, 0), this is e.g. useful to find only enemy bots
    pub fn iter_tile(&self, tile: Tile) -> impl Iterator<Item = Distance<Local>> + use<'_, Size> {
        (-(Size::R as i8)..=Size::R as i8).flat_map(move |dx| {
            (-(Size::R as i8)..=Size::R as i8)
                .filter(move |dy| {
                    self.at_unchecked(dx, *dy) == tile.to_char() && !(dx == 0 && *dy == 0)
                })
                .map(move |dy| Self::to_distance(dx, dy))
        })
    }

    /// iterate over scanned tiles excluding (0, 0)
    pub fn iter(&self) -> impl Iterator<Item = (Distance<Local>, Tile)> + use<'_, Size> {
        (-(Size::R as i8)..=Size::R as i8).flat_map(move |dx| {
            (-(Size::R as i8)..=Size::R as i8)
                .filter(move |dy| !(dx == 0 && *dy == 0))
                .map(move |dy| {
                    (
                        Self::to_distance(dx, dy),
                        Tile::from_char(self.at_unchecked(dx, dy)).unwrap(),
                    )
                })
        })
    }

    /// generate weak (does not block new scans) reference
    pub fn weak(&self) -> RadarScanWeak<Size> {
        RadarScanWeak {
            uuid: Guard::get_active_uuid(),
            _phantom: PhantomData,
        }
    }
}

impl<Size: RadarSize> Clone for RadarScan<Size> {
    fn clone(&self) -> Self {
        Guard::create_active()
    }
}

impl<Size: RadarSize> Drop for RadarScan<Size> {
    fn drop(&mut self) {
        Guard::decrease_count();
    }
}

/// A version of a radar scan that does not block new scans, but can't be used directly. Instead,
/// you can try to upgrade it to a full [`RadarScan`]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RadarScanWeak<Size: RadarSize> {
    uuid: u32,
    _phantom: PhantomData<Size>,
}

impl<Size: RadarSize> RadarScanWeak<Size> {
    pub fn upgrade(&self) -> Option<RadarScan<Size>> {
        Guard::try_create(self.uuid)
    }
}
