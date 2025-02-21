#![no_std]

use kartoffel_gps::Chunk;

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

pub fn get_global_pos(chunk: &Chunk<7>) -> Option<(u8, u8)> {
    UNIQUE_CHUNKS.get(chunk).cloned()
}

pub fn global_pos_entries() -> impl Iterator<Item = &'static Chunk<7>> {
    UNIQUE_CHUNKS.keys()
}
