use core::marker::PhantomData;

use async_kartoffel::Position;
use heapless::FnvIndexMap;

use crate::{error::OutOfMemory, Map};

use super::{Chunk, ChunkLocation, ChunkMap};

/// A map implementation based on 8 by 8 Chunks, stored in a hashmap.
pub struct ChunkMapHash<const N: usize, T, C: Chunk<T>> {
    data: FnvIndexMap<ChunkLocation, C, N>,
    _phantom: PhantomData<T>,
}
impl<const N: usize, T, C: Chunk<T>> ChunkMapHash<N, T, C> {
    pub fn new() -> Self {
        Self {
            data: FnvIndexMap::new(),
            _phantom: PhantomData,
        }
    }
}

impl<const N: usize, T, C: Chunk<T>> Default for ChunkMapHash<N, T, C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize, T, C: Chunk<T>> ChunkMap<T, C> for ChunkMapHash<N, T, C> {
    fn get_chunk_mut_or_new(&mut self, location: ChunkLocation) -> Result<&mut C, OutOfMemory> {
        if !self.data.contains_key(&location) {
            self.data
                .insert(location, C::new())
                .map_err(|_| OutOfMemory)?;
        }
        // unwrap: we just made sure it exists
        Ok(self.data.get_mut(&location).unwrap())
    }

    fn get_chunk(&self, location: ChunkLocation) -> Option<&C> {
        self.data.get(&location)
    }

    fn clear(&mut self) {
        self.data.clear()
    }
}

impl<const N: usize, T, C: Chunk<T>> Map<T> for ChunkMapHash<N, T, C> {
    fn set(&mut self, pos: Position, t: T) -> Result<(), T> {
        self.set_value(pos, t)
    }
    fn get(&self, pos: Position) -> Option<T> {
        self.get_value(pos)
    }
    fn clear(&mut self) {
        ChunkMap::clear(self)
    }
}
