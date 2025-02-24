use critical_section::Mutex;
use kartoffel::{is_radar_ready, radar_read, radar_scan};

use crate::{Coords, Local, Tile, Vec2};
use core::{
    cell::Cell,
    future::poll_fn,
    marker::PhantomData,
    num::NonZeroU64,
    ops::RangeInclusive,
    task::{Poll, Waker},
};

use super::{error::RadarError, Singleton};

#[non_exhaustive]
pub struct Radar;

pub(super) static mut RADAR: Singleton<Radar> = Singleton {
    instance: Some(Radar),
};

impl Radar {
    pub fn is_ready(&self) -> bool {
        is_radar_ready()
    }
    pub fn wait_blocking(&self) {
        while !self.is_ready() {
            //
        }
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
    pub fn try_scan<Size: RadarSize>(&mut self) -> Result<RadarScan<Size>, RadarError> {
        Guard::try_execute_scan::<Size>()?;
        Ok(Guard::create_active::<Size>())
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
    fn try_execute_scan<Size: RadarSize>() -> Result<(), RadarError> {
        Self::with_critical_section(|guard| {
            if guard.n_scans == 0 {
                if is_radar_ready() {
                    radar_scan(Size::D.into());

                    // can fail after u32::MAX iterations
                    guard.active_uuid = guard.active_uuid.wrapping_add(1);
                    Ok(())
                } else {
                    Err(RadarError::NotReady)
                }
            } else {
                Err(RadarError::AccessBlocked)
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
    const D: u8;
    fn range() -> RangeInclusive<i8> {
        -(Self::R as i8)..=Self::R as i8
    }
    fn contains(vec: Vec2<impl Coords>) -> bool {
        let (i1, i2) = vec.to_generic();
        -(Self::R as i16) <= i1
            && -(Self::R as i16) <= i2
            && (Self::R as i16) >= i1
            && (Self::R as i16) >= i2
    }
    fn to_str() -> &'static str;
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum D3 {}
impl private::Sealed for D3 {}
impl RadarSize for D3 {
    const R: u8 = 1;
    const D: u8 = 3;
    fn to_str() -> &'static str {
        "D3"
    }
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum D5 {}
impl private::Sealed for D5 {}
impl RadarSize for D5 {
    const R: u8 = 2;
    const D: u8 = 5;
    fn to_str() -> &'static str {
        "D5"
    }
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum D7 {}
impl private::Sealed for D7 {}
impl RadarSize for D7 {
    const R: u8 = 3;
    const D: u8 = 7;
    fn to_str() -> &'static str {
        "D7"
    }
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum D9 {}
impl private::Sealed for D9 {}
impl RadarSize for D9 {
    const R: u8 = 4;
    const D: u8 = 9;
    fn to_str() -> &'static str {
        "D9"
    }
}

#[non_exhaustive]
pub struct RadarScan<Size: RadarSize>(PhantomData<Size>);

impl<Size: RadarSize> RadarScan<Size> {
    #[inline(always)]
    fn radar_indices(vec: Vec2<Local>) -> Option<(i8, i8)> {
        let (dx, dy) = (vec.right(), vec.back());
        (dx.unsigned_abs() <= Size::R.into() && dy.unsigned_abs() <= Size::R.into())
            .then_some((dx as i8, dy as i8))
    }
    #[inline(always)]
    pub fn to_vec(dx: i8, dy: i8) -> Vec2<Local> {
        Vec2::new_local(dx.into(), (-dy).into())
    }

    pub fn contains(&self, vec: Vec2<Local>) -> bool {
        Self::radar_indices(vec).is_some()
    }

    pub fn at(&self, vec: Vec2<Local>) -> Option<Tile> {
        if let Some((dx, dy)) = Self::radar_indices(vec) {
            // unwrap: unknown tile means error
            Some(Tile::from_char(self.at_unchecked(dx, dy)).expect("encountered unknown tile"))
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn at_unchecked(&self, dx: i8, dy: i8) -> char {
        radar_read(Size::D.into(), dx, dy, 0) as u8 as char
    }

    #[inline(always)]
    pub fn bot_at(&self, vec: Vec2<Local>) -> Option<NonZeroU64> {
        if let Some((dx, dy)) = Self::radar_indices(vec) {
            let d1 = radar_read(Size::D.into(), dx, dy, 1) as u64;
            let d2 = radar_read(Size::D.into(), dx, dy, 2) as u64;
            NonZeroU64::new((d1 << 32) | d2)
        } else {
            None
        }
    }

    /// Scanned tiles matching tile excluding (0, 0), this is e.g. useful to find only enemy bots
    pub fn iter_tile(&self, tile: Tile) -> impl Iterator<Item = Vec2<Local>> + use<'_, Size> {
        Size::range().flat_map(move |dx| {
            Size::range()
                .filter(move |dy| {
                    self.at_unchecked(dx, *dy) == tile.to_char() && !(dx == 0 && *dy == 0)
                })
                .map(move |dy| Self::to_vec(dx, dy))
        })
    }

    /// iterate over scanned tiles excluding (0, 0)
    pub fn iter(&self) -> impl Iterator<Item = (Vec2<Local>, Tile)> + use<'_, Size> {
        Size::range().flat_map(move |dx| {
            Size::range()
                .filter(move |dy| !(dx == 0 && *dy == 0))
                .map(move |dy| {
                    (
                        Self::to_vec(dx, dy),
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

#[cfg(test)]
mod tests {

    use kartoffel::println;

    use super::*;
    use test_kartoffel::{
        assert, assert_eq, assert_err, assert_none, option_unwrap, result_unwrap, TestError,
    };

    #[test_case]
    fn guard() -> Result<(), TestError> {
        println!("testing guard");
        let mut radar = Radar;
        radar.wait_blocking();
        println!("scan working?");
        let scan: RadarScan<D3> = result_unwrap!(radar.try_scan());

        radar.wait_blocking();
        println!("new scan blocked?");
        assert_err!(radar.try_scan::<D3>(), RadarError::AccessBlocked);

        drop(scan);
        radar.wait_blocking();
        println!("new scan unblocked?");
        let scan: RadarScan<D3> = result_unwrap!(radar.try_scan());

        let weak = scan.weak();
        drop(scan);
        println!("weak upgrade?");
        let scan = option_unwrap!(weak.upgrade());
        drop(scan);

        radar.wait_blocking();
        println!("new scan unblocked?");
        let scan: RadarScan<D3> = result_unwrap!(radar.try_scan());
        drop(scan);
        println!("weak upgrade prevented?");
        assert_none!(weak.upgrade());

        Ok(())
    }

    #[test_case]
    fn guard() -> Result<(), TestError> {
        println!("testing iterators");
        let mut radar = Radar;

        fn test_iter<Size: RadarSize>(radar: &mut Radar) -> Result<(), TestError> {
            println!("{} scan", Size::D);
            radar.wait_blocking();
            let scan: RadarScan<Size> = result_unwrap!(radar.try_scan());
            println!("  iter");
            let n_tiles = (Size::D * Size::D - 1) as usize;
            assert_eq!(scan.iter().count(), n_tiles);
            let mut tiles = [[false; 9]; 9];
            for (vec, _) in scan.iter() {
                let dist_max = vec.front().unsigned_abs().max(vec.right().unsigned_abs());
                assert!(dist_max <= Size::R.into());

                tiles[(4 + vec.right()) as usize][(4 + vec.front()) as usize] = true;
            }
            assert_eq!(
                tiles
                    .iter()
                    .map(|row| row.iter().filter(|&&b| b).count())
                    .sum::<usize>(),
                n_tiles
            );

            Ok(())
        }

        test_iter::<D3>(&mut radar)?;
        test_iter::<D5>(&mut radar)?;
        test_iter::<D7>(&mut radar)?;
        test_iter::<D9>(&mut radar)?;

        Ok(())
    }
}
