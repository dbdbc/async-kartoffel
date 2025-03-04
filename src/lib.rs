#![no_std]

use async_kartoffel::Vec2;
use kartoffel_gps::{
    const_graph::Graph,
    gps::{MapSection, MapSectionTrait},
    map::TrueMap,
    GlobalPos,
};

const CHUNK_SIZE: usize = 7;

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

pub fn get_global_pos(chunk: &MapSection<CHUNK_SIZE>) -> Option<GlobalPos> {
    let (east, south) = UNIQUE_CHUNKS.get(chunk.compress()?.as_ref()).cloned()?;
    Some(GlobalPos::add_to_anchor(Vec2::new_global(
        i16::from(east),
        -i16::from(south),
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
    &BEACON_GRAPH.after(index)
}

pub fn map() -> &'static impl TrueMap {
    &TRUE_MAP
}
