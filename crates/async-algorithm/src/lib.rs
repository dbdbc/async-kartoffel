#![no_std]
#![no_main]
#![cfg_attr(feature = "test-kartoffel", feature(custom_test_frameworks))]
#![cfg_attr(feature = "test-kartoffel", reexport_test_harness_main = "test_main")]
#![cfg_attr(feature = "test-kartoffel", test_runner(test_kartoffel::runner))]

#[cfg(all(test, feature = "test-kartoffel"))]
#[no_mangle]
fn main() {
    test_main();
    loop {}
}

mod breakpoint;
mod chunk_map;
mod error;
mod exploration;
mod map;
mod math;
mod measure;
mod navigation;
mod stats;
mod terrain;

pub use breakpoint::Breakpoint;
pub use chunk_map::ChunkBool;
pub use chunk_map::ChunkIndex;
pub use chunk_map::ChunkLocation;
pub use chunk_map::ChunkMap;
pub use chunk_map::IterInChunk;
pub use exploration::Exploration;
pub use exploration::State as ExplorationState;
pub use map::Map;
pub use math::isqrt;
pub use measure::distance_walk_with_rotation;
pub use measure::DistanceBotStab;
pub use measure::DistanceBotWalk;
pub use measure::DistanceManhattan;
pub use measure::DistanceMax;
pub use measure::DistanceMeasure;
pub use measure::DistanceMin;
pub use navigation::Navigation;
pub use navigation::State as NavigationState;
pub use stats::StatsDog;
pub use terrain::ChunkTerrain;
pub use terrain::Terrain;
