use core::ops::Add;

use async_kartoffel::{Global, Position, Vec2};

use crate::error::OutOfMemory;

pub mod hash;

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
    fn to_vec(self) -> Vec2<Global> {
        let (east_in_chunk, south_in_chunk) = self.to_indices();
        Vec2::new_east_south(east_in_chunk as i16, south_in_chunk as i16)
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
        rhs.north_west_pos() + self.to_vec()
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

/// Chunk north-west corner is at east8 * 8, south8 * 8
#[derive(Clone, PartialEq, Eq, Ord, PartialOrd, Hash, Debug, Copy)]
pub struct ChunkLocation {
    east8: i16,
    south8: i16,
}
impl ChunkLocation {
    /// this is the 0, 0 (west-north) corner of the chunk
    pub fn north_west_pos(&self) -> Position {
        Position::add_to_anchor(Vec2::new_east_south(8 * self.east8, 8 * self.south8))
    }
    /// minimum distance vec to pos
    pub fn min_dist_to(&self, pos: Position) -> Vec2<Global> {
        let vec_anchor = pos - self.north_west_pos();
        fn dist_relaxed(dist_anchor: i16) -> i16 {
            match dist_anchor {
                ..0 => dist_anchor,
                0..8 => 0,
                8.. => dist_anchor - 7,
            }
        }

        Vec2::new_east_south(
            dist_relaxed(vec_anchor.east()),
            dist_relaxed(vec_anchor.south()),
        )
    }
}

pub fn to_chunk_pos(pos: Position) -> (ChunkLocation, ChunkIndex) {
    let vec = pos - Position::default();
    (
        ChunkLocation {
            east8: vec.east().div_euclid(8),
            south8: vec.south().div_euclid(8),
        },
        ChunkIndex::new(
            vec.east().rem_euclid(8) as u8,
            vec.south().rem_euclid(8) as u8,
        ),
    )
}

pub trait Chunk<T> {
    fn new() -> Self;
    fn get(&self, index: ChunkIndex) -> T;
    fn set(&mut self, index: ChunkIndex, t: T);
}

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

pub trait ChunkMap<T, C: Chunk<T>> {
    /// Return a mutable reference to the chunk at the given index. If it does not exist yet, it
    /// will be created first.
    fn get_chunk_mut_or_new(&mut self, location: ChunkLocation) -> Result<&mut C, OutOfMemory>;
    fn get_chunk(&self, location: ChunkLocation) -> Option<&C>;
    fn clear(&mut self);
    fn set_value(&mut self, pos: Position, t: T) -> Result<(), T> {
        let (div, rem) = to_chunk_pos(pos);
        match self.get_chunk_mut_or_new(div) {
            Ok(chunk) => {
                chunk.set(rem, t);
                Ok(())
            }
            Err(_) => Err(t),
        }
    }
    fn get_value(&self, pos: Position) -> Option<T> {
        let (div, rem) = to_chunk_pos(pos);
        self.get_chunk(div).map(|chunk| chunk.get(rem))
    }
}
