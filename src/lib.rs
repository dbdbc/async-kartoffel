#![no_std]

extern crate alloc;

use async_kartoffel::Vec2;
use kartoffel_gps::{
    beacon::{
        BeaconInfo, {NavigatorResources, NavigatorResourcesImpl},
    },
    const_graph::Graph,
    gps::{MapSection, MapSectionTrait},
    map::TrueMap,
    GlobalPos,
};

const CHUNK_SIZE: usize = 7;

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

pub fn get_global_pos(chunk: &MapSection<CHUNK_SIZE>) -> Option<GlobalPos> {
    let (east, south) = UNIQUE_CHUNKS.get(chunk.compress()?.as_ref()).cloned()?;
    Some(GlobalPos::add_to_anchor(Vec2::new_east_south(
        east.into(),
        south.into(),
    )))
}

pub fn global_pos_entries(
) -> impl Iterator<Item = &'static <MapSection<CHUNK_SIZE> as MapSectionTrait>::Compressed> {
    UNIQUE_CHUNKS.keys()
}

pub fn beacons() -> &'static [GlobalPos] {
    &BEACON_POSITIONS
}

pub fn beacon_graph() -> &'static impl Graph {
    &BEACON_GRAPH
}

pub fn beacons_before(index: u16) -> &'static [u16] {
    BEACON_GRAPH.after(index)
}

pub fn beacon_info() -> &'static BeaconInfo {
    &BEACON_INFO
}

pub fn map() -> &'static impl TrueMap {
    &TRUE_MAP
}

pub fn get_navigator_info() -> (usize, usize, usize, usize) {
    (
        NAV_MAX_PATH_LEN,
        NAV_MAX_ENTRY_EXIT,
        NAV_TRIV_BUFFER,
        NAV_NODE_BUFFER,
    )
}

/// Allocates the heap buffers used for the Navigator, and stores references to the map, beacons,
/// and beacon graph.
pub fn navigator_resources() -> impl NavigatorResources {
    NavigatorResourcesImpl::<
        NAV_MAX_PATH_LEN,
        NAV_MAX_ENTRY_EXIT,
        NAV_TRIV_BUFFER,
        NAV_NODE_BUFFER,
        NAV_ACTIVE_BUFFER,
        _,
        _,
    >::new(
        &TRUE_MAP,
        &BEACON_GRAPH,
        &BEACON_POSITIONS,
        u16::try_from(BEACON_INFO.max_beacon_dist).unwrap(),
    )
}
