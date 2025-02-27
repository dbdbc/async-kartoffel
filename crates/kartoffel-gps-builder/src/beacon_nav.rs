use crate::map::{Map, PositionBiMap};
use async_kartoffel::{Direction, Vec2};
use kartoffel_gps::GlobalPos;
use ndarray::{Array1, Array2, Axis};
use ndarray_stats::QuantileExt;

/// Trivial navigable means: No matter where you go, if the general direction is correct, you will
/// reach your target.
pub fn get_trivial_navigables(map: &Map, target: GlobalPos) -> Vec<GlobalPos> {
    let mut start = map.new_like();

    let mut ret: Vec<GlobalPos> = Default::default();

    if map.get(target) != Some(true) {
        return ret;
    }

    start.set(target, true).unwrap();
    ret.push(target);

    // fill for single direction
    for dir in Direction::all() {
        let mut i = 1i16;
        loop {
            let pos = target + Vec2::from_direction(dir, i);
            if map.get(pos) == Some(true) {
                start.set(pos, true).unwrap();
                ret.push(pos);
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
                any = any
                    || start.get(target + Vec2::from_direction(dir_ew, i_manhattan)) == Some(true);
                any = any
                    || start.get(target + Vec2::from_direction(dir_ns, i_manhattan)) == Some(true);

                for i_ew in 1..=i_manhattan - 1 {
                    let i_ns = i_manhattan - i_ew;
                    let pos = target
                        + Vec2::from_direction(dir_ew, i_ew)
                        + Vec2::from_direction(dir_ns, i_ns);
                    let neighbor_ew = pos - Vec2::from_direction(dir_ew, 1);
                    let neighbor_ns = pos - Vec2::from_direction(dir_ns, 1);

                    let walk = map.get(pos) == Some(true);
                    let triv_ns = start.get(neighbor_ns) == Some(true);
                    let triv_ew = start.get(neighbor_ew) == Some(true);
                    let wall_ns = map.get(neighbor_ns) != Some(true);
                    let wall_ew = map.get(neighbor_ew) != Some(true);

                    if walk && (triv_ns || wall_ns) && (triv_ew || wall_ew) && (triv_ew || triv_ns)
                    {
                        start.set(pos, true).unwrap();
                        ret.push(pos);
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
    ret
}

pub fn build_trivial_navigation_graph(map: &Map, positions: &PositionBiMap) -> Array2<u32> {
    let mut graph = Array2::<u32>::zeros((positions.len(), positions.len()));
    for (index_target, &pos) in positions.vec().iter().enumerate() {
        let trivials = get_trivial_navigables(&map, pos);

        for start in trivials {
            let index_start = positions.hashmap().get(&start).unwrap();
            graph[(index_target, *index_start)] = 1;
        }
    }
    graph
}

pub fn sub_graph(graph: &Array2<u32>, indices: &[usize]) -> Array2<u32> {
    let mut sub = Array2::zeros((indices.len(), indices.len()));
    for i1 in 0..indices.len() {
        for i2 in 0..indices.len() {
            sub[(i1, i2)] = graph[(indices[i1], indices[i2])];
        }
    }
    sub
}

pub fn build_distance_graph(positions: &PositionBiMap) -> Array2<u32> {
    let mut graph = Array2::<u32>::zeros((positions.len(), positions.len()));
    for (index1, &pos1) in positions.vec().iter().enumerate() {
        for (index2, &pos2) in positions.vec().iter().enumerate() {
            let delta = pos1 - pos2;
            graph[(index1, index2)] =
                u32::try_from(delta.east().abs() + delta.north().abs()).unwrap()
        }
    }
    graph
}

pub fn find_beacons(
    max_beacon_dist: u32,
    trivial_navigation_graph: &Array2<u32>,
    distance_graph: &Array2<u32>,
) -> Vec<usize> {
    let n = trivial_navigation_graph.len_of(Axis(0));
    assert!(trivial_navigation_graph.len_of(Axis(1)) == n);
    assert!(distance_graph.len_of(Axis(0)) == n);
    assert!(distance_graph.len_of(Axis(1)) == n);

    let symmetric_graph = &trivial_navigation_graph.view()
        * &trivial_navigation_graph.t()
        * distance_graph.mapv(|x| if x > max_beacon_dist { 0 } else { 1 });

    // using symmetric graph to ensure traversability of beacon graph in all directions
    // using product of new reachable and connections to existing beacons to ensure
    // interconnectivity of beacon graph
    // Restricting max distance of each beacon, to have a dependable criterion on which beacons
    // don't have be checked when calculating possible entry and exit beacons.
    let mut unreached_positions = Array1::ones(n);

    let mut beacons = Vec::<usize>::new();
    let mut beacon_arr = Array1::zeros(n);
    loop {
        let entry_exit = symmetric_graph.dot(&unreached_positions);
        let beacon_reachable = symmetric_graph.dot(&beacon_arr);

        let value = if beacons.len() > 0 {
            &entry_exit * &beacon_reachable
        } else {
            entry_exit.clone()
        };

        let best_index = value.argmax().unwrap();
        if value[best_index] == 0 {
            panic!("Could not find interconnected beacon graph");
        }
        beacons.push(best_index);

        let mut best_one_hot = Array1::zeros(n);
        best_one_hot[best_index] = 1;
        unreached_positions = unreached_positions * (1 - symmetric_graph.dot(&best_one_hot));

        beacon_arr = 1u32 - (1u32 - beacon_arr) * (1u32 - best_one_hot);

        if unreached_positions.iter().all(|&x| x == 0) {
            break;
        }
    }

    beacons
}
