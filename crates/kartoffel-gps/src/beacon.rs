use async_algorithm::{DistanceManhattan, DistanceMeasure, DistanceMin};
use async_kartoffel::{println, Direction, Vec2};
use heapless::Vec;

use crate::{const_graph::Graph, map::TrueMap, GlobalPos};

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

pub enum NavState {
    NoWorkDone,

    // currently determining possible exit nodes in range of destination
    Exit,

    // currently determining possible entry nodes in range of start
    Entry,

    // entry and exit nodes are known, currently calculating the route
    Route,

    // shortest path has been calculated
    Navigating,

    // after an update to start, confirm that the first node of the path is still reachable, if
    // thats not the case, repeat Entry and Route states
    ConfirmEntry,

    // after an update to destination, confirm the last node of the path can still see the
    // destination, if thats not the case, repeat Exit and Route states
    ConfirmExit,
}

pub struct Nav<
    const MAX_PATH_LEN: usize,
    const MAX_ACTIVE: usize,
    const MAX_ENTRY: usize,
    const MAX_EXIT: usize,
    T: TrueMap + 'static,
    G: Graph + 'static,
> {
    // const state
    map: &'static T,
    graph: &'static G,
    beacons: &'static [GlobalPos],
    info: &'static BeaconInfo,

    // computation state
    start: Option<GlobalPos>,
    entry_nodes: Option<Vec<usize, MAX_ENTRY>>,

    destination: Option<GlobalPos>,
    exit_nodes: Option<Vec<usize, MAX_EXIT>>,

    path: Option<Vec<GlobalPos, MAX_PATH_LEN>>,
    active: Vec<(GlobalPos, u16), MAX_ACTIVE>,
    state: NavState,
}

pub struct NavigationImpossible;

impl<
        const MAX_PATH_LEN: usize,
        const MAX_ACTIVE: usize,
        const MAX_ENTRY: usize,
        const MAX_EXIT: usize,
        T: TrueMap,
        G: Graph,
    > Nav<MAX_PATH_LEN, MAX_ACTIVE, MAX_ENTRY, MAX_EXIT, T, G>
{
    pub fn new(
        map: &'static T,
        graph: &'static G,
        beacons: &'static [GlobalPos],
        info: &'static BeaconInfo,
    ) -> Self {
        Self {
            map,
            graph,
            beacons,
            info,
            start: None,
            entry_nodes: None,
            destination: None,
            exit_nodes: None,
            path: None,
            active: Vec::new(),
            state: NavState::NoWorkDone,
        }
    }

    pub fn clear(&mut self) {
        todo!()
    }

    pub fn initialize(&mut self, start: GlobalPos, destination: GlobalPos) {
        self.start = Some(start);
        self.destination = Some(destination);
        self.entry_nodes = None;
        self.exit_nodes = None;
        self.path = None;
        self.active = Vec::new();
        self.state = NavState::NoWorkDone;
    }

    pub fn update_start(&mut self, start: GlobalPos) {
        todo!()
    }

    // TODO is it beneficial to keep this vs reinitializing
    pub fn update_destination(&mut self, destination: GlobalPos) {
        todo!()
    }

    pub fn compute(&mut self) {
        // if let (Some(start), Some(destination)) = (self.start, self.destination) {
        //     // entry
        //     let possible_entry_nodes = self
        //         .beacons
        //         .iter()
        //         .filter(|&pos| {
        //             u32::from(DistanceManhattan::measure(*pos - start)) <= self.info.max_beacon_dist
        //         })
        //         .filter(|&pos| is_navigation_trivial(self.map, start, pos));
        // }
        todo!()
    }

    pub fn is_good_dir(&self, direction: Direction) -> Option<bool> {
        todo!()
    }
}

/// assumes that start is walkable
/// assumes distances to be small enough to fit in i16
fn is_navigation_trivial<const BUFFER_SIZE: usize>(
    map: &impl TrueMap,
    start: GlobalPos,
    destination: GlobalPos,
) -> Result<bool, ()> {
    let vector = destination - start;
    let dirs = {
        let mut dirs = Vec::<Direction, 2>::new();
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

    if dirs.len() == 0 {
        // start == destination
        Ok(true)
    } else if dirs.len() == 1 {
        // straight line, example
        // S X X X D
        // dist_man: 4
        for i in 1..=i16::try_from(dist_man).unwrap() {
            if !map.get(start + Vec2::from_direction(dirs[0], i)) {
                return Ok(false);
            }
        }
        Ok(true)
    } else {
        let max_dir_0 = u16::try_from(vector.get(dirs[0])).unwrap();
        let max_dir_1 = dist_man - max_dir_0;
        // let (dir_smaller, dir_larger) = {
        //     let dist_d0 = vector.get(dirs[0]);
        //     let dist_d1 = vector.get(dirs[1]);
        //     if dist_d0 >= dist_d1 {
        //         (dirs[0], dirs[1])
        //     } else {
        //         (dirs[1], dirs[0])
        //     }
        // };

        // out of bounds
        if usize::try_from(dist_min + 1).unwrap() >= BUFFER_SIZE {
            return Err(());
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
            start
                + Vec2::from_direction(dirs[0], i16::try_from(dist_dir_0 - i_index).unwrap())
                + Vec2::from_direction(dirs[1], i16::try_from(dist_dir_1 + i_index).unwrap())
        };

        let neighbor_indices = |i_manhattan: u16, i_index: u16| {
            let i_0 = i_manhattan.min(max_dir_0) - i_index;
            let i_1 = i_manhattan - i_0;

            let index_offset = if i_manhattan >= max_dir_0 { 1 } else { 0 };

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
                    if next_indices.len() == 0 {
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

    use super::*;
    use test_kartoffel::{
        assert, assert_eq, assert_err, assert_none, option_unwrap, result_unwrap, TestError,
    };

    pub struct TestMap<const WIDTH: usize, const HEIGHT: usize> {
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

    #[test_case]
    fn trivial_nav_positive() -> Result<(), TestError> {
        let mut rng = {
            let seed = [0u8; 32];
            let rng = SmallRng::from_seed(seed);
            rng
        };

        fn for_size<const WIDTH: usize, const HEIGHT: usize>(
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

                let triv_nav = is_navigation_trivial::<64>(&map, start, dest).unwrap();

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
                }
            }

            assert_eq!(map.dirty_outside.get(), false);
            println!();

            Ok(())
        }

        for i in 0..100 {
            println!("\n{}", i);
            for_size::<1, 5>(&mut rng, (4, 1))?;
            for_size::<2, 5>(&mut rng, (4, 1))?;
            for_size::<3, 5>(&mut rng, (4, 1))?;
            for_size::<4, 5>(&mut rng, (4, 1))?;
            for_size::<5, 5>(&mut rng, (4, 1))?;
            for_size::<5, 4>(&mut rng, (4, 1))?;
            for_size::<5, 3>(&mut rng, (4, 1))?;
            for_size::<5, 2>(&mut rng, (4, 1))?;
            for_size::<5, 1>(&mut rng, (4, 1))?;
        }

        Ok(())
    }
}
