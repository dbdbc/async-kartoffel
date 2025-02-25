#![no_std]

use async_kartoffel::{Global, Vec2};
use kartoffel_gps::gps::{Chunk, MapSection};

const CHUNK_SIZE: usize = 7;

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

pub fn get_global_pos(chunk: &Chunk<CHUNK_SIZE>) -> Option<(u8, u8)> {
    UNIQUE_CHUNKS.get(chunk.compress()?.as_ref()).cloned()
}

pub fn global_pos_entries(
) -> impl Iterator<Item = &'static <Chunk<CHUNK_SIZE> as MapSection>::Compressed> {
    UNIQUE_CHUNKS.keys()
}

pub fn beacons() -> &'static [Vec2<Global>] {
    &BEACON_POSITIONS
}

pub fn beacons_size() -> u16 {
    BEACON_GRAPH.size()
}

pub fn beacons_after(index: u16) -> &'static [u16] {
    &BEACON_GRAPH.after(index)
}

pub fn beacons_before(index: u16) -> &'static [u16] {
    &BEACON_GRAPH.after(index)
}
