/// Data structure has run out of available memory
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct OutOfMemory;

/// Maps are expected to be extended (unknown areas become known), but already known tiles must not
/// change, for [`super::exploration::Exploration`] and [`super::terrain::ChunkTerrain`].
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct MapInconsistent;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum MapError {
    OutOfMemory,
    MapInconsistent,
}
impl From<OutOfMemory> for MapError {
    fn from(_: OutOfMemory) -> Self {
        MapError::OutOfMemory
    }
}
impl From<MapInconsistent> for MapError {
    fn from(_: MapInconsistent) -> Self {
        MapError::MapInconsistent
    }
}

/// tried to change the navigations start position, but there is not currently a navigation running
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NoTarget;
