use heapless::{FnvIndexSet, Vec};

use async_kartoffel::{Direction, Global, Position, RadarScan, RadarSize, Rotation, Vec2};

use crate::{chunk_map::to_chunk_pos, Breakpoint};

use super::{
    chunk_map::{Chunk, ChunkIndex, ChunkLocation, ChunkMap},
    error::{MapError, MapInconsistent},
};

#[derive(Clone, Copy, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub enum Terrain {
    /// no information available about this tile
    Unknown,
    /// tile cannot be walked on
    Blocked,
    /// tile can be walked on, but it is not known whether the tile is reachable / can be reached
    /// from the current location
    Walkable,
    /// tile can be walked on and can be reached
    Reachable,
}

impl Terrain {
    pub fn from_walkable(walkable: bool) -> Self {
        match walkable {
            true => Self::Walkable,
            false => Self::Blocked,
        }
    }
    pub fn is_walkable(self) -> Option<bool> {
        match self {
            Terrain::Unknown => None,
            Terrain::Blocked => Some(false),
            Terrain::Walkable => Some(true),
            Terrain::Reachable => Some(true),
        }
    }
    pub fn is_reachable(self) -> Option<bool> {
        match self {
            Terrain::Unknown => None,
            Terrain::Blocked => Some(false),
            Terrain::Walkable => None,
            Terrain::Reachable => Some(true),
        }
    }
    pub fn is_known_walkable(self) -> bool {
        match self {
            Terrain::Unknown => false,
            Terrain::Blocked => false,
            Terrain::Walkable => true,
            Terrain::Reachable => true,
        }
    }
    fn from_last_bits(byte: u8) -> Self {
        match byte {
            0b00 => Self::Unknown,
            0b01 => Self::Blocked,
            0b10 => Self::Walkable,
            0b11 => Self::Reachable,
            _ => unreachable!(),
        }
    }
    fn to_last_bits(self) -> u8 {
        match self {
            Self::Unknown => 0b00,
            Self::Blocked => 0b01,
            Self::Walkable => 0b10,
            Self::Reachable => 0b11,
        }
    }
    // index ranging from 0..4
    fn get_in_byte(byte: u8, index: u8) -> Self {
        Self::from_last_bits((byte >> (index * 2)) & 0b11)
    }
    fn set_in_byte(self, byte: u8, index: u8) -> u8 {
        let current_shifted = byte & (0b11 << (index * 2));
        let bits_shifted = self.to_last_bits() << (index * 2);
        let xor = current_shifted ^ bits_shifted;
        byte ^ xor
    }
}

#[derive(Clone)]
/// memory efficient Chunk for Terrain
pub struct ChunkTerrain {
    value: [u8; 16],
}
impl Chunk<Terrain> for ChunkTerrain {
    fn get(&self, index: ChunkIndex) -> Terrain {
        let i1 = index.index64().div_euclid(4);
        let i2 = index.index64().rem_euclid(4);
        Terrain::get_in_byte(self.value[i1 as usize], i2)
    }

    fn set(&mut self, index: ChunkIndex, item: Terrain) {
        let i1 = index.index64().div_euclid(4);
        let i2 = index.index64().rem_euclid(4);
        self.value[i1 as usize] = item.set_in_byte(self.value[i1 as usize], i2);
    }

    fn new() -> Self {
        Self {
            // all zeros -> all unknown
            value: Default::default(),
        }
    }
}

impl ChunkTerrain {
    /// center: relative to south west corner (0, 0 - corner in in_chunk coords)
    /// Fails if a tile would be changed from an already known state. This can happen, if we tried
    /// to walked into another bot, and is probably really annoying to repair.
    async fn update_from_radar<Size: RadarSize>(
        &mut self,
        radar: &RadarScan<Size>,
        center: Vec2<Global>,
        direction: Direction,
    ) -> Result<Self, MapInconsistent> {
        let r: i16 = Size::R as i16;
        let mut new_self = self.clone();
        let mut map_changed = false;
        for east in (center.east() - r).clamp(0, 7)..=(center.east() + r).clamp(0, 7) {
            for north in (center.north() - r).clamp(0, 7)..=(center.north() + r).clamp(0, 7) {
                let vec_from_center = Vec2::new_global(east, north) - center;
                // unwrap okay, because we ensured it is in radar range
                let walkable = radar
                    .at(vec_from_center.local(direction))
                    .unwrap()
                    .is_walkable_terrain();
                let in_chunk_index = ChunkIndex::new(east as u8, north as u8);
                match new_self.get(in_chunk_index).is_walkable() {
                    None => new_self.set(in_chunk_index, Terrain::from_walkable(walkable)),
                    Some(current_walkable) => {
                        if current_walkable != walkable {
                            map_changed = true;
                        }
                    }
                }
            }
        }
        match map_changed {
            true => Err(MapInconsistent),
            false => Ok(new_self),
        }
    }
}

pub async fn update_chunk_map<M: ChunkMap<Terrain, ChunkTerrain>, Size: RadarSize>(
    map: &mut M,
    radar: &RadarScan<Size>,
    pos: Position,
    direction: Direction,
) -> Result<(), MapError> {
    let vec = Vec2::new_global(Size::R as i16, Size::R as i16);
    // unique chunks, since maximum scan size is 9 the scan is guaranteed to fit into 4 chunks
    let locations: FnvIndexSet<ChunkLocation, 4> = Rotation::all()
        .into_iter()
        .map(|rot| to_chunk_pos(pos + vec.rotate(rot)).0)
        .collect();

    let mut results = Vec::<ChunkTerrain, 4>::new();
    for &location in locations.iter() {
        let chunk = map.get_chunk_mut_or_new(location)?;
        let in_chunk_coords = pos - location.south_west_pos();
        let updated = chunk
            .update_from_radar(radar, in_chunk_coords, direction)
            .await?;
        // can't fail
        _ = results.push(updated);
        Breakpoint::new().await;
    }

    // only write updates once we are sure they are consistent with the map
    for (&location, update) in locations.into_iter().zip(results) {
        // unwrap(): we already ensured it exists
        *map.get_chunk_mut_or_new(location).unwrap() = update;
    }
    Ok(())
}
