use core::{fmt::Display, hash::Hash};

use async_kartoffel::{Direction, Global, RadarScan, RadarSize, Vec2, D3, D5, D7, D9};

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct OutOfBounds;

pub trait MapSectionTrait: Default + Hash + PartialEq + Eq + PartialOrd + Ord + Clone {
    type Size: RadarSize;
    type Compressed: phf_shared::FmtConst + phf_shared::PhfHash + Eq + Hash;
    fn compressed_type() -> &'static str;
    fn from_scan(scan: &RadarScan<Self::Size>, facing: Direction) -> Self {
        let mut ret = Self::default();
        for i_east in Self::Size::range() {
            for i_north in Self::Size::range() {
                let vec = Vec2::new_global(i_east.into(), i_north.into());
                // unwrap: we know vec is in bounds
                let walkable = scan.at(vec.local(facing)).unwrap().is_walkable_terrain();
                // unwrap: we know vec is in bounds
                ret.set(vec, walkable).unwrap();
            }
        }
        ret
    }
    fn from_function(f: impl Fn(Vec2<Global>) -> bool) -> Self {
        let mut ret = Self::default();
        for i_east in Self::Size::range() {
            for i_south in Self::Size::range() {
                let vec = Vec2::new_east(i_east.into()) + Vec2::new_south(i_south.into());
                // unwrap: we know vec is in bounds
                ret.set(vec, f(vec)).unwrap();
            }
        }
        ret
    }

    /// Convert into a representation using minimal memory space. This representation must be
    /// unique and it must fail if the center tile is false (not walkable).
    fn compress(&self) -> Option<Self::Compressed>;
    fn from_compressed(compressed: Self::Compressed) -> Self;
    fn get(&self, vec: Vec2<Global>) -> Result<bool, OutOfBounds>;
    fn set(&mut self, vec: Vec2<Global>, val: bool) -> Result<(), OutOfBounds>;
    fn at_center(&self) -> bool {
        self.get(Vec2::default()).unwrap()
    }
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Clone)]
pub struct MapSection<const N: usize>([[bool; N]; N]);

impl<const N: usize> Display for MapSection<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for line in self.0 {
            for val in line {
                write!(f, "{}", if val { "." } else { "#" })?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl<const N: usize> Default for MapSection<N> {
    fn default() -> Self {
        MapSection([[false; N]; N])
    }
}

impl MapSectionTrait for MapSection<3> {
    type Size = D3;
    type Compressed = [u8; 1];

    fn compressed_type() -> &'static str {
        "[u8; 1]"
    }

    fn compress(&self) -> Option<Self::Compressed> {
        // unwrap: we known center is in range
        if self.get(Vec2::default()).unwrap() {
            let mut ret = Self::Compressed::default();
            let mut index = 0usize;
            for i_east in Self::Size::range() {
                for i_north in Self::Size::range() {
                    if i_east != 0 || i_north != 0 {
                        if self
                            .get(Vec2::new_global(i_east.into(), i_north.into()))
                            .unwrap()
                        {
                            ret[index.div_euclid(8)] ^= 1u8 << index.rem_euclid(8);
                        }
                        index += 1;
                    }
                }
            }
            Some(ret)
        } else {
            None
        }
    }

    fn from_compressed(compressed: Self::Compressed) -> Self {
        let mut ret = Self::default();
        let mut index: usize = 0;
        for i_east in Self::Size::range() {
            for i_north in Self::Size::range() {
                if i_east != 0 || i_north != 0 {
                    let val = compressed[index.div_euclid(8)] & (1u8 << index.rem_euclid(8)) > 0u8;
                    // unwrap: always in bounds
                    ret.set(Vec2::new_global(i_east.into(), i_north.into()), val)
                        .unwrap();

                    index += 1;
                }
            }
        }
        // unwrap: center is always in bounds
        ret.set(Vec2::default(), true).unwrap();
        ret
    }

    fn get(&self, vec: Vec2<Global>) -> Result<bool, OutOfBounds> {
        if Self::Size::contains(vec) {
            Ok(
                self.0[usize::try_from(vec.east() + i16::from(Self::Size::R)).unwrap()]
                    [usize::try_from(vec.north() + i16::from(Self::Size::R)).unwrap()],
            )
        } else {
            Err(OutOfBounds)
        }
    }

    fn set(&mut self, vec: Vec2<Global>, val: bool) -> Result<(), OutOfBounds> {
        if Self::Size::contains(vec) {
            self.0[usize::try_from(vec.east() + i16::from(Self::Size::R)).unwrap()]
                [usize::try_from(vec.north() + i16::from(Self::Size::R)).unwrap()] = val;
            Ok(())
        } else {
            Err(OutOfBounds)
        }
    }
}

impl MapSectionTrait for MapSection<5> {
    type Size = D5;
    type Compressed = [u8; 3];

    fn compressed_type() -> &'static str {
        "[u8; 3]"
    }

    fn compress(&self) -> Option<Self::Compressed> {
        // unwrap: we known center is in range
        if self.get(Vec2::default()).unwrap() {
            let mut ret = Self::Compressed::default();
            let mut index = 0usize;
            for i_east in Self::Size::range() {
                for i_north in Self::Size::range() {
                    if i_east != 0 || i_north != 0 {
                        if self
                            .get(Vec2::new_global(i_east.into(), i_north.into()))
                            .unwrap()
                        {
                            ret[index.div_euclid(8)] ^= 1u8 << index.rem_euclid(8);
                        }
                        index += 1;
                    }
                }
            }
            Some(ret)
        } else {
            None
        }
    }

    fn from_compressed(compressed: Self::Compressed) -> Self {
        let mut ret = Self::default();
        let mut index: usize = 0;
        for i_east in Self::Size::range() {
            for i_north in Self::Size::range() {
                if i_east != 0 || i_north != 0 {
                    let val = compressed[index.div_euclid(8)] & (1u8 << index.rem_euclid(8)) > 0u8;
                    // unwrap: always in bounds
                    ret.set(Vec2::new_global(i_east.into(), i_north.into()), val)
                        .unwrap();

                    index += 1;
                }
            }
        }
        // unwrap: center is always in bounds
        ret.set(Vec2::default(), true).unwrap();
        ret
    }

    fn get(&self, vec: Vec2<Global>) -> Result<bool, OutOfBounds> {
        if Self::Size::contains(vec) {
            Ok(
                self.0[usize::try_from(vec.east() + i16::from(Self::Size::R)).unwrap()]
                    [usize::try_from(vec.north() + i16::from(Self::Size::R)).unwrap()],
            )
        } else {
            Err(OutOfBounds)
        }
    }

    fn set(&mut self, vec: Vec2<Global>, val: bool) -> Result<(), OutOfBounds> {
        if Self::Size::contains(vec) {
            self.0[usize::try_from(vec.east() + i16::from(Self::Size::R)).unwrap()]
                [usize::try_from(vec.north() + i16::from(Self::Size::R)).unwrap()] = val;
            Ok(())
        } else {
            Err(OutOfBounds)
        }
    }
}

impl MapSectionTrait for MapSection<7> {
    type Size = D7;
    type Compressed = [u8; 6];

    fn compressed_type() -> &'static str {
        "[u8; 6]"
    }

    fn compress(&self) -> Option<Self::Compressed> {
        // unwrap: we known center is in range
        if self.get(Vec2::default()).unwrap() {
            let mut ret = Self::Compressed::default();
            let mut index = 0usize;
            for i_east in Self::Size::range() {
                for i_north in Self::Size::range() {
                    if i_east != 0 || i_north != 0 {
                        if self
                            .get(Vec2::new_global(i_east.into(), i_north.into()))
                            .unwrap()
                        {
                            ret[index.div_euclid(8)] ^= 1u8 << index.rem_euclid(8);
                        }
                        index += 1;
                    }
                }
            }
            Some(ret)
        } else {
            None
        }
    }

    fn from_compressed(compressed: Self::Compressed) -> Self {
        let mut ret = Self::default();
        let mut index: usize = 0;
        for i_east in Self::Size::range() {
            for i_north in Self::Size::range() {
                if i_east != 0 || i_north != 0 {
                    let val = compressed[index.div_euclid(8)] & (1u8 << index.rem_euclid(8)) > 0u8;
                    // unwrap: always in bounds
                    ret.set(Vec2::new_global(i_east.into(), i_north.into()), val)
                        .unwrap();

                    index += 1;
                }
            }
        }
        // unwrap: center is always in bounds
        ret.set(Vec2::default(), true).unwrap();
        ret
    }

    fn get(&self, vec: Vec2<Global>) -> Result<bool, OutOfBounds> {
        if Self::Size::contains(vec) {
            Ok(
                self.0[usize::try_from(vec.east() + i16::from(Self::Size::R)).unwrap()]
                    [usize::try_from(vec.north() + i16::from(Self::Size::R)).unwrap()],
            )
        } else {
            Err(OutOfBounds)
        }
    }

    fn set(&mut self, vec: Vec2<Global>, val: bool) -> Result<(), OutOfBounds> {
        if Self::Size::contains(vec) {
            self.0[usize::try_from(vec.east() + i16::from(Self::Size::R)).unwrap()]
                [usize::try_from(vec.north() + i16::from(Self::Size::R)).unwrap()] = val;
            Ok(())
        } else {
            Err(OutOfBounds)
        }
    }
}

impl MapSectionTrait for MapSection<9> {
    type Size = D9;
    type Compressed = [u8; 10];

    fn compressed_type() -> &'static str {
        "[u8; 10]"
    }

    fn compress(&self) -> Option<Self::Compressed> {
        // unwrap: we known center is in range
        if self.get(Vec2::default()).unwrap() {
            let mut ret = Self::Compressed::default();
            let mut index = 0usize;
            for i_east in Self::Size::range() {
                for i_north in Self::Size::range() {
                    if i_east != 0 || i_north != 0 {
                        if self
                            .get(Vec2::new_global(i_east.into(), i_north.into()))
                            .unwrap()
                        {
                            ret[index.div_euclid(8)] ^= 1u8 << index.rem_euclid(8);
                        }
                        index += 1;
                    }
                }
            }
            Some(ret)
        } else {
            None
        }
    }

    fn from_compressed(compressed: Self::Compressed) -> Self {
        let mut ret = Self::default();
        let mut index: usize = 0;
        for i_east in Self::Size::range() {
            for i_north in Self::Size::range() {
                if i_east != 0 || i_north != 0 {
                    let val = compressed[index.div_euclid(8)] & (1u8 << index.rem_euclid(8)) > 0u8;
                    // unwrap: always in bounds
                    ret.set(Vec2::new_global(i_east.into(), i_north.into()), val)
                        .unwrap();

                    index += 1;
                }
            }
        }
        // unwrap: center is always in bounds
        ret.set(Vec2::default(), true).unwrap();
        ret
    }

    fn get(&self, vec: Vec2<Global>) -> Result<bool, OutOfBounds> {
        if Self::Size::contains(vec) {
            Ok(
                self.0[usize::try_from(vec.east() + i16::from(Self::Size::R)).unwrap()]
                    [usize::try_from(vec.north() + i16::from(Self::Size::R)).unwrap()],
            )
        } else {
            Err(OutOfBounds)
        }
    }

    fn set(&mut self, vec: Vec2<Global>, val: bool) -> Result<(), OutOfBounds> {
        if Self::Size::contains(vec) {
            self.0[usize::try_from(vec.east() + i16::from(Self::Size::R)).unwrap()]
                [usize::try_from(vec.north() + i16::from(Self::Size::R)).unwrap()] = val;
            Ok(())
        } else {
            Err(OutOfBounds)
        }
    }
}
