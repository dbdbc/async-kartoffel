use core::{convert::identity, marker::PhantomData};

use alloc::boxed::Box;
use async_algorithm::{DistanceManhattan, DistanceMeasure, DistanceMin};
use async_kartoffel::{Direction, Vec2};

use heapless::{binary_heap::Min, BinaryHeap, Vec};

use crate::{const_graph::Graph, map::TrueMap, GlobalPos};

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Clone, Copy)]
pub enum NavigatorError {
    OutOfMemory,
    Uninitialized,
    NavigationImpossible,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Hash)]
pub struct BeaconInfo {
    pub max_beacon_dist: u32,

    /// from an arbitrary position, how many beacons are in sight (maximum)?
    pub max_beacons_entry: u32,

    /// for an arbitrary position, from how many beacons can it maximally be reached?
    pub max_beacons_exit: u32,

    /// what is the largest number of steps between any beacons
    pub max_path_length: u32,

    /// number of beacons
    pub n_beacons: u32,
}

// TODO
// states: Uninit {}
//         Computing {start, dest}
//         Failed {start, dest}
//         Ready {start, dest, path}
//         PartialReady {start, dest, path}: start has been moved to an invalid place
// pub enum NavigatorReady {
//     Ready,
//     Partial,
// }
// pub enum NavigatorComputation {
//     Computing,
//     Failed,
//     Ready {
//         ready: NavigatorReady,
//         path: Option<Vec<u16, MAX_PATH_LEN>>, // path in reverse order
//     },
// }
// pub enum NavigatorState {
//     Uninitialized,
//     Initialized {
//         start: GlobalPos,
//         destination: GlobalPos,
//         computation: NavigatorComputation,
//     },
// }

pub trait Navigator {
    fn initialize(&mut self, start: GlobalPos, destination: GlobalPos); // Uninit -> Computing

    fn reset(&mut self); // any -> Uninit

    fn get_start(&self) -> Option<GlobalPos>; // Computing, Ready, PartialReady, Failed

    fn get_destination(&self) -> Option<GlobalPos>; // Computing, Ready, PartialReady, Failed

    fn compute(&mut self) -> Result<(), NavigatorError>; // Computing -> Failed, Ready

    fn is_ready(&self) -> bool; // state == Ready

    fn is_completed(&self) -> Option<bool>; // Ready

    fn move_start_to(&mut self, new_start: GlobalPos) -> Result<(), NavigatorError>; // Ready, PartialReady -> Ready, PartialReady

    fn is_dir_good(&self, dir: Direction) -> Option<bool>; // Ready

    fn good_dirs(&self) -> Option<Vec<Direction, 2>>; // Ready
}

// Ord derived implementation ensures desired `Min` behaviour for the priority queue
#[derive(PartialEq, Eq, Debug)]
struct NavActiveEntry {
    estimated_cost: u16,
    past_cost: u16,
    node: Node,
}

impl PartialOrd for NavActiveEntry {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NavActiveEntry {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        match self.estimated_cost.cmp(&other.estimated_cost) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.past_cost.cmp(&other.past_cost) {
            core::cmp::Ordering::Equal => {}
            ord => return ord.reverse(), // note the reverse, because we want to prioritize paths
                                         // that already crossed a larger distance
        }
        self.node.cmp(&other.node)
    }
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash)]
enum Node {
    Beacon(u16),
    Start,
    Destination,
}

// Buffers that can be used for computations
struct NavigatorBuffers<const MAX_ENTRY_EXIT: usize, const NODE_BUFFER: usize> {
    entry_nodes: Box<Vec<u16, MAX_ENTRY_EXIT>>,
    exit_nodes: Box<Vec<u16, MAX_ENTRY_EXIT>>,
    active: Box<BinaryHeap<NavActiveEntry, Min, NODE_BUFFER>>,
    node_info: Box<[Option<(u16, Node)>; NODE_BUFFER]>,
}

impl<const MAX_ENTRY_EXIT: usize, const NODE_BUFFER: usize> Default
    for NavigatorBuffers<MAX_ENTRY_EXIT, NODE_BUFFER>
{
    fn default() -> Self {
        Self {
            entry_nodes: Default::default(),
            exit_nodes: Default::default(),
            active: Default::default(),
            node_info: Box::new([None; NODE_BUFFER]),
        }
    }
}

/// const navigator state
pub struct NavigatorStatic<T: TrueMap + 'static, G: Graph + 'static> {
    map: &'static T,
    graph: &'static G,
    beacons: &'static [GlobalPos],
    max_beacon_dist: u16,
}

impl<T: TrueMap, G: Graph> Clone for NavigatorStatic<T, G> {
    fn clone(&self) -> Self {
        Self {
            map: self.map,
            graph: self.graph,
            beacons: self.beacons,
            max_beacon_dist: self.max_beacon_dist,
        }
    }
}

pub struct NavigatorImpl<
    const MAX_PATH_LEN: usize,
    const MAX_ENTRY_EXIT: usize,
    const TRIV_BUFFER: usize,
    const NODE_BUFFER: usize,
    T: TrueMap + 'static,
    G: Graph + 'static,
> {
    const_state: NavigatorStatic<T, G>,

    // intermediate buffers
    buffers: NavigatorBuffers<MAX_ENTRY_EXIT, NODE_BUFFER>,

    _phantom: PhantomData<[(); TRIV_BUFFER]>,

    // config
    start: Option<GlobalPos>,
    destination: Option<GlobalPos>,

    // computation result
    path: Option<Vec<u16, MAX_PATH_LEN>>, // path in reverse order
}

impl<
        const MAX_PATH_LEN: usize,
        const MAX_ENTRY_EXIT: usize,
        const TRIV_BUFFER: usize,
        const NODE_BUFFER: usize,
        T: TrueMap,
        G: Graph,
    > Navigator for NavigatorImpl<MAX_PATH_LEN, MAX_ENTRY_EXIT, TRIV_BUFFER, NODE_BUFFER, T, G>
{
    fn initialize(&mut self, start: GlobalPos, destination: GlobalPos) {
        self.start = Some(start);
        self.destination = Some(destination);
        self.path = None;
    }

    fn reset(&mut self) {
        self.start = None;
        self.destination = None;
        self.path = None;
    }

    fn get_start(&self) -> Option<GlobalPos> {
        self.start
    }

    fn get_destination(&self) -> Option<GlobalPos> {
        self.destination
    }

    fn compute(&mut self) -> Result<(), NavigatorError> {
        if let (Some(start), Some(destination)) = (self.start, self.destination) {
            self.path = Some(Vec::new());
            compute::<MAX_PATH_LEN, MAX_ENTRY_EXIT, TRIV_BUFFER, NODE_BUFFER>(
                start,
                destination,
                &mut self.buffers,
                self.const_state.clone(),
                self.path.as_mut().unwrap(), // unwrap: we just created path
            )
        } else {
            Err(NavigatorError::Uninitialized)
        }
    }

    fn is_ready(&self) -> bool {
        self.start.is_some() && self.destination.is_some() && self.path.is_some()
    }

    fn is_completed(&self) -> Option<bool> {
        Some(self.start? == self.destination?)
    }

    fn move_start_to(&mut self, new_start: GlobalPos) -> Result<(), NavigatorError> {
        let good_dirs = self.good_dirs().ok_or(NavigatorError::Uninitialized)?;

        let path = self.path.as_mut().ok_or(NavigatorError::Uninitialized)?;
        let old_start = self.start.as_mut().ok_or(NavigatorError::Uninitialized)?;
        let destination = self.destination.ok_or(NavigatorError::Uninitialized)?;

        // no movement
        if new_start == *old_start {
            return Ok(());
        }

        let movement = new_start - *old_start;
        let movement_dirs = {
            let mut v = Vec::<Direction, 2>::new();
            // unwraps: there can only be two directions to move to
            match movement.get(Direction::East) {
                ..0 => v.push(Direction::West).unwrap(),
                0 => {}
                1.. => v.push(Direction::East).unwrap(),
            }
            match movement.get(Direction::North) {
                ..0 => v.push(Direction::South).unwrap(),
                0 => {}
                1.. => v.push(Direction::North).unwrap(),
            }
            v
        };
        let movement_steps = DistanceManhattan::measure(movement);

        let target = if let Some(&node) = path.last() {
            self.const_state.beacons[usize::from(node)]
        } else {
            destination
        };

        if new_start == target {
            // reached target
            _ = path.pop();
        } else if movement_steps == 1 && good_dirs.contains(movement_dirs.first().unwrap()) {
            // unwrap: there is exactly one dir
            // preferred case: single step in good dir, is really easy because of trivial
            // navigation rules
            *old_start = new_start;
        } else {
            // new calculation needed :( but to save cost, only to target
            compute::<MAX_PATH_LEN, MAX_ENTRY_EXIT, TRIV_BUFFER, NODE_BUFFER>(
                new_start,
                target,
                &mut self.buffers,
                self.const_state.clone(),
                path,
            )?;
        }
        *old_start = new_start;
        Ok(())
    }

    // checks if the next tile in dir is walkable, and if this is the right direction
    fn is_dir_good(&self, dir: Direction) -> Option<bool> {
        let path = self.path.as_ref()?;
        let start = self.start?;
        let destination = self.destination?;

        let target = if let Some(&node) = path.last() {
            self.const_state.beacons[usize::from(node)]
        } else {
            destination
        };

        Some(
            (target - start).get(dir) > 0
                && self
                    .const_state
                    .map
                    .get(start + Vec2::from_direction(dir, 1)),
        )
    }

    fn good_dirs(&self) -> Option<Vec<Direction, 2>> {
        let path = self.path.as_ref()?;
        let start = self.start?;
        let destination = self.destination?;

        let target = if let Some(&node) = path.last() {
            self.const_state.beacons[usize::from(node)]
        } else {
            destination
        };

        let movement = target - start;

        let mut v = Vec::<Direction, 2>::new();

        let mut check_and_add = |dir: Direction| {
            if self
                .const_state
                .map
                .get(start + Vec2::from_direction(dir, 1))
            {
                // unwrap: there can only be two dirs to add
                v.push(dir).unwrap();
            }
        };

        match movement.get(Direction::East) {
            ..0 => check_and_add(Direction::West),
            0 => {}
            1.. => check_and_add(Direction::East),
        }
        match movement.get(Direction::North) {
            ..0 => check_and_add(Direction::South),
            0 => {}
            1.. => check_and_add(Direction::North),
        }

        Some(v)
    }
}

impl<
        const MAX_PATH_LEN: usize,
        const MAX_ENTRY_EXIT: usize,
        const TRIV_BUFFER: usize,
        const NODE_BUFFER: usize,
        T: TrueMap,
        G: Graph,
    > NavigatorImpl<MAX_PATH_LEN, MAX_ENTRY_EXIT, TRIV_BUFFER, NODE_BUFFER, T, G>
{
    pub fn new(
        map: &'static T,
        graph: &'static G,
        beacons: &'static [GlobalPos],
        max_beacon_dist: u16,
    ) -> Self {
        Self {
            const_state: NavigatorStatic {
                map,
                graph,
                beacons,
                max_beacon_dist,
            },
            start: None,
            destination: None,
            path: None,
            buffers: NavigatorBuffers::default(),
            _phantom: Default::default(),
        }
    }
}

/// assumes that start is walkable
/// assumes distances to be small enough to fit in i16
/// BUFFER_SIZE needs to be at least (dist_manhattan + 2).div_floor(2)
/// this functions makes no assumptions about BUFFER_SIZE, but returns an Err
fn is_navigation_trivial<const BUFFER_SIZE: usize>(
    map: &impl TrueMap,
    start: GlobalPos,
    destination: GlobalPos,
) -> Result<bool, NavigatorError> {
    let vector = destination - start;
    let dirs = {
        let mut dirs = Vec::<Direction, 2>::new();
        // unwrap: there can only be two dirs to add
        match vector.east() {
            ..=-1 => dirs.push(Direction::West).unwrap(),
            0 => (),
            1.. => dirs.push(Direction::East).unwrap(),
        }
        match vector.north() {
            ..=-1 => dirs.push(Direction::South).unwrap(),
            0 => (),
            1.. => dirs.push(Direction::North).unwrap(),
        }
        dirs
    };

    let dist_man = DistanceManhattan::measure(vector);
    let dist_min = DistanceMin::measure(vector);

    if dirs.is_empty() {
        // start == destination
        Ok(true)
    } else if dirs.len() == 1 {
        // straight line, example
        // S X X X D
        // dist_man: 4
        // unwrap: all distances are expected to fit in i16
        for i in 1..=i16::try_from(dist_man).unwrap() {
            if !map.get(start + Vec2::from_direction(dirs[0], i)) {
                return Ok(false);
            }
        }
        Ok(true)
    } else {
        // unwrap: all distances are expected to fit in i16, vector in dir is nonnegative
        let max_dir_0 = u16::try_from(vector.get(dirs[0])).unwrap();
        let max_dir_1 = dist_man - max_dir_0;

        // out of bounds
        if usize::from(dist_min + 1) > BUFFER_SIZE {
            return Err(NavigatorError::OutOfMemory);
        };

        let mut actives_next = [false; BUFFER_SIZE];
        actives_next[0] = true;

        //   1 2 3 3 3
        //  / / / / / 2
        // S X X X X /1
        // X X X X X /
        // X X X X D
        // dist_min: 2
        // dist_man: 6

        // returns index_th position in rect with given distance to start
        let pos_from_manhattan_and_index = |i_manhattan: u16, i_index: u16| {
            let dist_dir_0 = i_manhattan.min(max_dir_0);
            let dist_dir_1 = i_manhattan - dist_dir_0;
            // unwrap: distances fit in i16
            start
                + Vec2::from_direction(dirs[0], i16::try_from(dist_dir_0 - i_index).unwrap())
                + Vec2::from_direction(dirs[1], i16::try_from(dist_dir_1 + i_index).unwrap())
        };

        let neighbor_indices = |i_manhattan: u16, i_index: u16| {
            let i_0 = i_manhattan.min(max_dir_0) - i_index;
            let i_1 = i_manhattan - i_0;

            let index_offset = if i_manhattan >= max_dir_0 { 1 } else { 0 };

            // unwrap: there can only be two neighbors
            let mut next = Vec::<u16, 2>::new();
            if i_0 < max_dir_0 {
                next.push(i_index - index_offset).unwrap();
            }
            if i_1 < max_dir_1 {
                next.push(i_index + 1 - index_offset).unwrap();
            }
            next
        };

        let mut actives: [bool; BUFFER_SIZE];
        for i_manhattan in 0..dist_man {
            actives = actives_next;
            actives_next = [false; BUFFER_SIZE];

            // number of locations on this diagonal that might be active
            let n_to_check = dist_min.min(i_manhattan).min(dist_man - i_manhattan) + 1;

            for index_to_check in 0..n_to_check {
                if actives[usize::from(index_to_check)] {
                    let next_indices: Vec<u16, 2> = neighbor_indices(i_manhattan, index_to_check)
                        .into_iter()
                        .filter(|&i| {
                            let pos = pos_from_manhattan_and_index(i_manhattan + 1, i);
                            map.get(pos)
                        })
                        .collect();
                    if next_indices.is_empty() {
                        // we reached a dead end
                        return Ok(false);
                    }
                    for i in next_indices {
                        actives_next[usize::from(i)] = true;
                    }
                }
            }
        }
        Ok(true)
    }
}

fn compute<
    const MAX_PATH_LEN: usize,
    const MAX_ENTRY_EXIT: usize,
    const TRIV_BUFFER: usize,
    const NODE_BUFFER: usize,
>(
    start: GlobalPos,
    destination: GlobalPos,
    buffers: &mut NavigatorBuffers<MAX_ENTRY_EXIT, NODE_BUFFER>,
    const_state: NavigatorStatic<impl TrueMap, impl Graph>,
    path: &mut Vec<u16, MAX_PATH_LEN>, // path in reverse order, calculation is appended, TODO can
                                       // no longer prevents overflow errors reliably now
) -> Result<(), NavigatorError> {
    // clear intermediate state
    *buffers = Default::default();
    let mut node_info_destination: Option<(u16, Node)> = None;

    // entry
    *buffers.entry_nodes = const_state
        .beacons
        .iter()
        .enumerate()
        .filter(|(_, &pos)| DistanceManhattan::measure(pos - start) <= const_state.max_beacon_dist)
        .filter(|(_, &pos)| {
            // possible OutOfMemory error ignored here, but thats ok because it can only appear
            // if TRIV_BUFFER is misconfigured
            is_navigation_trivial::<TRIV_BUFFER>(const_state.map, start, pos).is_ok_and(identity)
        })
        .map(|(index, _)| u16::try_from(index).unwrap())
        .collect();

    // exit
    *buffers.exit_nodes = const_state
        .beacons
        .iter()
        .enumerate()
        .filter(|(_, &pos)| {
            DistanceManhattan::measure(destination - pos) <= const_state.max_beacon_dist
        })
        .filter(|(_, &pos)| {
            is_navigation_trivial::<TRIV_BUFFER>(const_state.map, pos, destination).unwrap()
        }) // TODO unwrap
        .map(|(index, _)| u16::try_from(index).unwrap())
        .collect();

    if start == destination
        || (DistanceManhattan::measure(destination - start) <= const_state.max_beacon_dist
            && is_navigation_trivial::<TRIV_BUFFER>(const_state.map, start, destination)
                .map_err(|_| NavigatorError::OutOfMemory)?)
    {
        // nothing to add to path, navigation from start to destination is trivial
    } else {
        // graph initialization
        for &node_index in &*buffers.entry_nodes {
            let pos = const_state.beacons[usize::from(node_index)];
            let past_cost = DistanceManhattan::measure(pos - start);

            buffers
                .active
                .push(NavActiveEntry {
                    estimated_cost: past_cost + DistanceManhattan::measure(destination - pos),
                    past_cost,
                    node: Node::Beacon(node_index),
                })
                .map_err(|_| NavigatorError::OutOfMemory)?;
            buffers.node_info[usize::from(node_index)] = Some((past_cost, Node::Start));
        }

        // graph traversal
        'main_loop: while let Some(NavActiveEntry {
            estimated_cost: _,
            past_cost,
            node,
        }) = buffers.active.pop()
        {
            match node {
                Node::Start => unreachable!("start is never added to the active nodes"),
                Node::Destination => {
                    break 'main_loop;
                }
                Node::Beacon(node_index) => {
                    let pos = const_state.beacons[usize::from(node_index)];

                    // this check ensures that nodes that were added multiple time are only
                    // processed once and might not be necessary
                    if buffers.node_info[usize::from(node_index)]
                        .is_none_or(|(past_cost_ni, _)| past_cost_ni == past_cost)
                    {
                        // neighbor is destination node
                        if buffers
                            .exit_nodes
                            .iter()
                            .any(|&exit_node| exit_node == node_index)
                        {
                            let total_cost =
                                past_cost + DistanceManhattan::measure(destination - pos);
                            if let Some((total_cost_old, parent)) = &mut node_info_destination {
                                if total_cost < *total_cost_old {
                                    *total_cost_old = total_cost;
                                    *parent = node;
                                }

                                buffers
                                    .active
                                    .push(NavActiveEntry {
                                        estimated_cost: total_cost,
                                        past_cost: total_cost,
                                        node: Node::Destination,
                                    })
                                    .unwrap();
                            } else {
                                node_info_destination = Some((total_cost, node));
                                buffers
                                    .active
                                    .push(NavActiveEntry {
                                        estimated_cost: total_cost,
                                        past_cost: total_cost,
                                        node: Node::Destination,
                                    })
                                    .unwrap();
                            }
                        }

                        // neighbors are beacon nodes
                        for &neighbor in const_state.graph.after(node_index) {
                            let pos_neighbor = const_state.beacons[usize::from(neighbor)];
                            let past_cost_neighbor =
                                past_cost + DistanceManhattan::measure(pos_neighbor - pos);
                            if let Some((past_cost_old, parent)) =
                                &mut buffers.node_info[usize::from(neighbor)]
                            {
                                if past_cost_neighbor < *past_cost_old {
                                    *past_cost_old = past_cost_neighbor;
                                    *parent = node;
                                    buffers
                                        .active
                                        .push(NavActiveEntry {
                                            estimated_cost: past_cost_neighbor
                                                + DistanceManhattan::measure(
                                                    destination - pos_neighbor,
                                                ),
                                            past_cost: past_cost_neighbor,
                                            node: Node::Beacon(neighbor),
                                        })
                                        .unwrap();
                                }
                            } else {
                                buffers.node_info[usize::from(neighbor)] =
                                    Some((past_cost_neighbor, node));
                                buffers
                                    .active
                                    .push(NavActiveEntry {
                                        estimated_cost: past_cost_neighbor
                                            + DistanceManhattan::measure(
                                                destination - pos_neighbor,
                                            ),
                                        past_cost: past_cost_neighbor,
                                        node: Node::Beacon(neighbor),
                                    })
                                    .unwrap();
                            }
                        }
                    }
                }
            };
        }

        // path calculation
        if let Some((_cost, mut parent_node)) = node_info_destination {
            loop {
                match parent_node {
                    Node::Start => break,
                    Node::Destination => unreachable!(),
                    Node::Beacon(beacon_index) => {
                        path.push(beacon_index)
                            .map_err(|_| NavigatorError::OutOfMemory)?;
                        // the node must have appeared while traversing the graph
                        parent_node = buffers.node_info[usize::from(beacon_index)].unwrap().1;
                    }
                }
            }
        } else {
            return Err(NavigatorError::NavigationImpossible);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use core::{
        cell::Cell,
        fmt::{Display, Write},
    };

    use async_kartoffel::{print, println, Global};
    use rand::{
        distr::{Distribution, Uniform},
        rngs::SmallRng,
        seq::IteratorRandom,
        SeedableRng,
    };

    use crate::pos::pos_east_north;

    extern crate alloc;

    use super::*;
    use test_kartoffel::{
        assert, assert_eq, assert_err, assert_none, option_unwrap, result_unwrap, TestError,
    };

    struct TestMap<const WIDTH: usize, const HEIGHT: usize> {
        tiles: [[bool; WIDTH]; HEIGHT],
        dirty_outside: Cell<bool>,
    }

    impl<const WIDTH: usize, const HEIGHT: usize> Display for TestMap<WIDTH, HEIGHT> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            for i_h in 0..HEIGHT {
                for i_w in 0..WIDTH {
                    f.write_char(if self.tiles[i_h][i_w] { '.' } else { '#' })?
                }
                f.write_char('\n')?;
            }
            core::fmt::Result::Ok(())
        }
    }

    impl<const WIDTH: usize, const HEIGHT: usize> TestMap<WIDTH, HEIGHT> {
        fn new_like(&self) -> Self {
            Self {
                tiles: [[false; WIDTH]; HEIGHT],
                dirty_outside: Cell::new(false),
            }
        }

        fn new(tiles: [[bool; WIDTH]; HEIGHT]) -> Self {
            Self {
                tiles,
                dirty_outside: Cell::new(false),
            }
        }

        fn corner_north_west(&self) -> GlobalPos {
            pos_east_north(0, 0)
        }

        fn corner_north_east(&self) -> GlobalPos {
            pos_east_north(WIDTH as i16 - 1, 0)
        }

        fn corner_south_west(&self) -> GlobalPos {
            pos_east_north(0, -(HEIGHT as i16) + 1)
        }

        fn corner_south_east(&self) -> GlobalPos {
            pos_east_north(WIDTH as i16 - 1, -(HEIGHT as i16) + 1)
        }

        fn set(&mut self, pos: GlobalPos, val: bool) -> Result<(), ()> {
            let vec = pos.sub_anchor();
            let east = vec.east();
            let south = vec.south();
            if east < 0 || east >= self.width() as i16 || south < 0 || south >= self.height() as i16
            {
                Err(())
            } else {
                self.tiles[south as usize][east as usize] = val;
                Ok(())
            }
        }
    }

    impl<const WIDTH: usize, const HEIGHT: usize> TrueMap for TestMap<WIDTH, HEIGHT> {
        fn get(&self, pos: GlobalPos) -> bool {
            let vec = pos.sub_anchor();
            let east = vec.east();
            let south = vec.south();
            if east < 0 || east >= self.width() as i16 || south < 0 || south >= self.height() as i16
            {
                self.dirty_outside.set(true);
                false
            } else {
                self.tiles[south as usize][east as usize]
            }
        }

        fn vec_east(&self) -> Vec2<Global> {
            Vec2::new_east(WIDTH as i16)
        }

        fn vec_south(&self) -> Vec2<Global> {
            Vec2::new_south(HEIGHT as i16)
        }

        fn width(&self) -> u16 {
            WIDTH as u16
        }

        fn height(&self) -> u16 {
            HEIGHT as u16
        }
    }

    #[test_case]
    fn trivial_nav1() -> Result<(), TestError> {
        println!("map1");
        let map = TestMap::new([
            [true, true, true, true, true, true],
            [true, true, true, true, true, true],
            [true, true, true, true, true, true],
            [true, true, true, true, true, true],
            [true, true, true, true, true, true],
            [true, true, true, true, true, true],
        ]);
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_north_west(), map.corner_south_east()),
            Ok(true)
        );
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_south_east(), map.corner_north_west()),
            Ok(true)
        );
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_south_west(), map.corner_north_east()),
            Ok(true)
        );
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_north_east(), map.corner_south_west()),
            Ok(true)
        );
        assert_eq!(map.dirty_outside.get(), false);

        Ok(())
    }

    #[test_case]
    fn trivial_nav2() -> Result<(), TestError> {
        println!("map2");
        let map = TestMap::new([
            [true, true, true, true, true, true],
            [true, false, true, true, false, true],
            [true, true, false, false, true, true],
            [true, false, true, true, false, true],
            [true, true, true, true, true, true],
        ]);
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_north_west(), map.corner_south_east()),
            Ok(false)
        );
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_south_east(), map.corner_north_west()),
            Ok(false)
        );
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_south_west(), map.corner_north_east()),
            Ok(false)
        );
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_north_east(), map.corner_south_west()),
            Ok(false)
        );
        assert_eq!(map.dirty_outside.get(), false);

        Ok(())
    }

    #[test_case]
    fn trivial_nav3() -> Result<(), TestError> {
        println!("map3");
        let map = TestMap::new([
            [true, true, true],
            [true, true, true],
            [true, true, true],
            [true, false, true],
        ]);
        assert_eq!(map.get(pos_east_north(2, -3)), true);
        assert_eq!(map.get(pos_east_north(1, -3)), false);
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_north_west(), map.corner_south_east()),
            Ok(false)
        );
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_south_east(), map.corner_north_west()),
            Ok(true)
        );
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_south_west(), map.corner_north_east()),
            Ok(true)
        );
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_north_east(), map.corner_south_west()),
            Ok(false)
        );
        assert_eq!(map.dirty_outside.get(), false);

        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_north_east(), pos_east_north(3, -3)),
            Ok(false)
        );
        assert_eq!(map.dirty_outside.get(), true);

        Ok(())
    }

    /// Trivial navigable means: No matter where you go, if the general direction is correct, you will
    /// reach your destination.
    fn is_trivial_navigable_test<const WIDTH: usize, const HEIGHT: usize>(
        map: &TestMap<WIDTH, HEIGHT>,
        xx_start: GlobalPos,
        destination: GlobalPos,
    ) -> bool {
        let mut start = map.new_like();

        if !map.get(destination) {
            return false;
        }

        start.set(destination, true).unwrap();
        if destination == xx_start {
            return true;
        }

        // fill for single direction
        for dir in Direction::all() {
            let mut i = 1i16;
            loop {
                let pos = destination + Vec2::from_direction(dir, i);
                if map.get(pos) {
                    start.set(pos, true).unwrap();
                    if pos == xx_start {
                        return true;
                    }
                    i += 1;
                } else {
                    break;
                }
            }
        }

        for dir_ew in [Direction::East, Direction::West] {
            for dir_ns in [Direction::North, Direction::South] {
                // recursive condition:
                // - all walkable neighbors in the two directions must be trivially navigable from
                // - at least on neighbor in the two directions must be walkable
                // => all walkable neighbors in the two directions must have been checked before
                // => diagonal iteration with increasing manhattan dist

                let mut i_manhattan = 2i16;
                loop {
                    let mut any: bool = false;
                    any = any || start.get(destination + Vec2::from_direction(dir_ew, i_manhattan));
                    any = any || start.get(destination + Vec2::from_direction(dir_ns, i_manhattan));

                    for i_ew in 1..=i_manhattan - 1 {
                        let i_ns = i_manhattan - i_ew;
                        let pos = destination
                            + Vec2::from_direction(dir_ew, i_ew)
                            + Vec2::from_direction(dir_ns, i_ns);
                        let neighbor_ew = pos - Vec2::from_direction(dir_ew, 1);
                        let neighbor_ns = pos - Vec2::from_direction(dir_ns, 1);

                        let walk = map.get(pos);
                        let triv_ns = start.get(neighbor_ns);
                        let triv_ew = start.get(neighbor_ew);
                        let wall_ns = !map.get(neighbor_ns);
                        let wall_ew = !map.get(neighbor_ew);

                        if walk
                            && (triv_ns || wall_ns)
                            && (triv_ew || wall_ew)
                            && (triv_ew || triv_ns)
                        {
                            start.set(pos, true).unwrap();
                            if pos == xx_start {
                                return true;
                            }
                            any = true;
                        }
                    }
                    if !any {
                        break;
                    }
                    i_manhattan += 1;
                }
            }
        }
        return false;
    }

    #[test_case]
    fn trivial_nav_positive() -> Result<(), TestError> {
        let mut rng = {
            let seed = [0u8; 32];
            let rng = SmallRng::from_seed(seed);
            rng
        };

        // (max_dist + 2).div_ceil(2).next_power_of_two(),
        fn for_size<const WIDTH: usize, const HEIGHT: usize, const BUFFER_SIZE: usize>(
            rng: &mut SmallRng,
            odds: (u16, u16),
        ) -> Result<(), TestError> {
            let mut map = [[false; WIDTH]; HEIGHT];
            let dist = Uniform::try_from(0..odds.0 + odds.1).unwrap();

            for i_h in 0..HEIGHT {
                for i_w in 0..WIDTH {
                    map[i_h][i_w] = dist.sample(rng) < odds.0;
                }
            }

            let map = TestMap::new(map);

            println!("{}", map);

            for i in 0..4 {
                let (start, dest, dir0, dir1) = match i {
                    0 => (
                        map.corner_north_west(),
                        map.corner_south_east(),
                        Direction::South,
                        Direction::East,
                    ),
                    1 => (
                        map.corner_south_west(),
                        map.corner_north_east(),
                        Direction::North,
                        Direction::East,
                    ),
                    2 => (
                        map.corner_south_east(),
                        map.corner_north_west(),
                        Direction::North,
                        Direction::West,
                    ),
                    3 => (
                        map.corner_north_east(),
                        map.corner_south_west(),
                        Direction::South,
                        Direction::West,
                    ),
                    _ => unreachable!(),
                };

                let triv_nav = is_navigation_trivial::<BUFFER_SIZE>(&map, start, dest).unwrap();

                if triv_nav {
                    println!("t {:?} {:?}", dir0, dir1);
                    let mut pos = start;
                    for i in 0..WIDTH + HEIGHT - 2 {
                        let dirs = [dir0, dir1].into_iter().filter(|&dir| {
                            let new_pos = pos + Vec2::from_direction(dir, 1);
                            (dest - new_pos).get(dir) >= 0 && map.get(new_pos)
                        });
                        let dir = dirs.choose(rng);
                        if let Some(dir) = dir {
                            pos = pos + Vec2::from_direction(dir, 1);
                        } else {
                            return Err(TestError);
                        }
                    }
                    assert_eq!(pos, dest);
                    assert_eq!(
                        is_trivial_navigable_test(&map, start, dest) || !map.get(start),
                        true
                    );
                } else {
                    println!("n {:?} {:?}", dir0, dir1);
                    assert_eq!(is_trivial_navigable_test(&map, start, dest), false);
                }
            }

            // assert_eq!(map.dirty_outside.get(), false);
            println!();

            Ok(())
        }

        for i in 0..100 {
            println!("\n{}", i);
            for_size::<1, 5, 3>(&mut rng, (4, 1))?;
            for_size::<2, 5, 3>(&mut rng, (4, 1))?;
            for_size::<3, 5, 4>(&mut rng, (4, 1))?;
            for_size::<4, 5, 4>(&mut rng, (4, 1))?;
            for_size::<5, 5, 5>(&mut rng, (4, 1))?;
            for_size::<5, 4, 4>(&mut rng, (4, 1))?;
            for_size::<5, 3, 4>(&mut rng, (4, 1))?;
            for_size::<5, 2, 3>(&mut rng, (4, 1))?;
            for_size::<5, 1, 3>(&mut rng, (4, 1))?;
        }

        Ok(())
    }
}
