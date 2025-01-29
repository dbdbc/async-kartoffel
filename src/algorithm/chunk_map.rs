use core::{marker::PhantomData, ops::Add};

use crate::{Distance, Error, Global, Position};
use heapless::FnvIndexMap;

use super::map::Map;

/// Location in chunk, between (0, 0)..=(7, 7)
#[derive(Clone, PartialEq, Eq, Ord, PartialOrd, Hash, Debug, Copy)]
pub struct ChunkIndex {
    /// in 0..64
    index: u8,
}
impl ChunkIndex {
    pub fn new(i1: u8, i2: u8) -> Self {
        assert!(i1 < 8);
        assert!(i2 < 8);
        Self { index: i1 * 8 + i2 }
    }
    pub fn to_indices(self) -> (u8, u8) {
        assert!(self.index < 64);
        (self.index.div_euclid(8), self.index.rem_euclid(8))
    }
    pub fn index64(self) -> u8 {
        self.index
    }
    fn to_dist(self) -> Distance<Global> {
        let (east_in_chunk, north_in_chunk) = self.to_indices();
        Distance::new_global(east_in_chunk as i16, north_in_chunk as i16)
    }
    fn increase_by_one(self) -> Option<Self> {
        assert!(self.index < 64);
        if self.index >= 63 {
            None
        } else {
            Some(Self {
                index: self.index.saturating_add(1),
            })
        }
    }
    fn first() -> Self {
        Self { index: 0 }
    }
}
impl Add<ChunkLocation> for ChunkIndex {
    type Output = Position;

    fn add(self, rhs: ChunkLocation) -> Self::Output {
        rhs.south_west_pos() + self.to_dist()
    }
}

pub struct IterInChunk {
    in_chunk_index: Option<ChunkIndex>,
}
impl Default for IterInChunk {
    fn default() -> Self {
        Self::new()
    }
}
impl IterInChunk {
    pub fn new() -> Self {
        Self {
            in_chunk_index: Some(ChunkIndex::first()),
        }
    }
}
impl Iterator for IterInChunk {
    type Item = ChunkIndex;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(in_chunk) = self.in_chunk_index {
            self.in_chunk_index = in_chunk.increase_by_one();
            Some(in_chunk)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self
            .in_chunk_index
            .map_or(0, |index| 64 - index.index)
            .into();
        (size, Some(size))
    }
}
impl ExactSizeIterator for IterInChunk {}

/// Chunk south-west corner is at east8 * 8, north8 * 8
#[derive(Clone, PartialEq, Eq, Ord, PartialOrd, Hash, Debug, Copy)]
pub struct ChunkLocation {
    east8: i16,
    north8: i16,
}
impl ChunkLocation {
    /// this is the 0, 0 (west-south) corner of the chunk
    pub fn south_west_pos(&self) -> Position {
        Position::from_from_origin(Distance::new_global(8 * self.east8, 8 * self.north8))
    }
    /// minimum distance to pos
    pub fn min_dist_to(&self, pos: Position) -> Distance<Global> {
        let dist_anchor = pos - self.south_west_pos();
        fn dist_relaxed(dist_anchor: i16) -> i16 {
            match dist_anchor {
                ..0 => dist_anchor,
                0..8 => 0,
                8.. => dist_anchor - 7,
            }
        }

        Distance::new_global(
            dist_relaxed(dist_anchor.east()),
            dist_relaxed(dist_anchor.north()),
        )
    }
}

pub trait Chunk<T> {
    fn new() -> Self;
    fn get(&self, index: ChunkIndex) -> T;
    fn set(&mut self, index: ChunkIndex, t: T);
}

/// A map implementation based on 8 by 8 Chunks, stored in a hashmap.
pub struct ChunkMap<const N: usize, T, C: Chunk<T>> {
    data: FnvIndexMap<ChunkLocation, C, N>,
    _phantom: PhantomData<T>,
}
impl<const N: usize, T, C: Chunk<T>> ChunkMap<N, T, C> {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn to_chunk_pos(pos: Position) -> (ChunkLocation, ChunkIndex) {
        let dist = pos - Position::default();
        (
            ChunkLocation {
                east8: dist.east().div_euclid(8),
                north8: dist.north().div_euclid(8),
            },
            ChunkIndex::new(
                dist.east().rem_euclid(8) as u8,
                dist.north().rem_euclid(8) as u8,
            ),
        )
    }
    /// Return a mutable reference to the chunk at the given index. If it does not exist yet, it
    /// will be created first.
    pub fn get_mut_chunk_or_new(&mut self, index: ChunkLocation) -> Result<&mut C, Error> {
        if !self.data.contains_key(&index) {
            self.data
                .insert(index, C::new())
                .map_err(|_| Error::OutOfMemory)?;
        }
        // unwrap: we just made sure it exists
        Ok(self.data.get_mut(&index).unwrap())
    }
}
impl<const N: usize, T, C: Chunk<T>> Default for ChunkMap<N, T, C> {
    fn default() -> Self {
        Self {
            data: FnvIndexMap::new(),
            _phantom: PhantomData,
        }
    }
}
impl<const N: usize, T, C: Chunk<T>> Map<T> for ChunkMap<N, T, C> {
    fn set(&mut self, pos: Position, t: T) -> Result<(), T> {
        let (div, rem) = Self::to_chunk_pos(pos);
        match self.get_mut_chunk_or_new(div) {
            Ok(chunk) => {
                chunk.set(rem, t);
                Ok(())
            }
            Err(_) => Err(t),
        }
    }
    fn get(&self, pos: Position) -> Option<T> {
        let (div, rem) = Self::to_chunk_pos(pos);
        self.data.get(&div).map(|chunk| chunk.get(rem))
    }
    fn clear(&mut self) {
        self.data.clear()
    }
}

// Chunk impl

/// Most basic [`Chunk`]
impl<T: Clone + Default> Chunk<T> for [[T; 8]; 8] {
    fn new() -> Self {
        Default::default()
    }
    fn get(&self, index: ChunkIndex) -> T {
        let (i1, i2) = index.to_indices();
        self[i1 as usize][i2 as usize].clone()
    }
    fn set(&mut self, index: ChunkIndex, t: T) {
        let (i1, i2) = index.to_indices();
        self[i1 as usize][i2 as usize] = t;
    }
}

/// Memory efficient [`Chunk`] of bool
#[derive(Clone, PartialEq, Eq, Ord, PartialOrd, Hash, Debug, Copy)]
pub struct ChunkBool {
    data: u64,
}
impl ChunkBool {
    fn bit(index: ChunkIndex) -> u64 {
        1u64 << index.index64()
    }
    pub fn get(self, index: ChunkIndex) -> bool {
        (self.data & Self::bit(index)) > 0
    }
    pub fn set(&mut self, index: ChunkIndex, val: bool) {
        if val {
            self.data |= Self::bit(index);
        } else {
            self.data &= !Self::bit(index);
        }
    }
}
