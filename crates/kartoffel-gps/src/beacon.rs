use async_kartoffel::Direction;

use crate::{const_graph::Graph, map::TrueMap, GlobalPos};

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Hash)]
pub struct BeaconInfo {
    pub max_beacon_dist: u32,

    /// from an arbitrary position, how many beacons are in sight (maximum)?
    pub max_beacons_entry: u32,

    /// for an arbitrary position, from how many beacons can it maximally be reached?
    pub max_beacons_exit: u32,

    pub max_path_length: u32,
}

pub struct Nav<const N: usize, T: TrueMap + 'static, G: Graph + 'static> {
    map: &'static T,
    graph: &'static G,
    beacons: &'static [GlobalPos],
}

pub struct NavigationImpossible;

impl<const N: usize, T: TrueMap, G: Graph> Nav<N, T, G> {
    pub fn initialize(&mut self, start: GlobalPos, destination: GlobalPos) {
        todo!()
    }

    pub fn update_start(&mut self, destination: GlobalPos) {
        todo!()
    }

    pub fn update_destination(&mut self, destination: GlobalPos) {
        todo!()
    }

    pub fn compute(&mut self) {
        todo!()
    }

    pub fn is_right_dir(&self, pos: GlobalPos, direction: Direction) -> Option<bool> {
        todo!()
    }
}
