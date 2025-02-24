#![no_std]

use kartoffel_gps::{Chunk, MapSection};

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

pub fn get_global_pos(chunk: &Chunk<9>) -> Option<(u8, u8)> {
    UNIQUE_CHUNKS.get(chunk.compress()?.as_ref()).cloned()
}

pub fn global_pos_entries() -> impl Iterator<Item = &'static <Chunk<9> as MapSection>::Compressed> {
    UNIQUE_CHUNKS.keys()
}
