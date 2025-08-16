use crate::{
    graph::{Graph, GraphMappingSingleNode},
    map::{Map, PositionBiMap},
};
use async_kartoffel_generic::{Direction, Vec2};
use kartoffel_gps::{GlobalPos, beacon::BeaconInfo};
use ndarray_stats::QuantileExt;

pub type PosGraph<'s, 'd> = Graph<GlobalPos, GlobalPos, &'s PositionBiMap, &'d PositionBiMap>;

fn distance(a: GlobalPos, b: GlobalPos) -> u16 {
    let v = a - b;
    v.east().unsigned_abs() + v.south().unsigned_abs()
}

/// Trivial navigable means: No matter where you go, if the general direction is correct, you will
/// reach your destination.
pub fn get_trivial_navigables(map: &Map, destination: GlobalPos) -> Vec<GlobalPos> {
    let mut start = map.new_like();

    let mut ret: Vec<GlobalPos> = Default::default();

    if !map.get(destination) {
        return ret;
    }

    start.set(destination, true).unwrap();
    ret.push(destination);

    // fill for single direction
    for dir in Direction::all() {
        let mut i = 1i16;
        loop {
            let pos = destination + Vec2::new_in_direction(dir, i);
            if map.get(pos) {
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
                any = any || start.get(destination + Vec2::new_in_direction(dir_ew, i_manhattan));
                any = any || start.get(destination + Vec2::new_in_direction(dir_ns, i_manhattan));

                for i_ew in 1..=i_manhattan - 1 {
                    let i_ns = i_manhattan - i_ew;
                    let pos = destination
                        + Vec2::new_in_direction(dir_ew, i_ew)
                        + Vec2::new_in_direction(dir_ns, i_ns);
                    let neighbor_ew = pos - Vec2::new_in_direction(dir_ew, 1);
                    let neighbor_ns = pos - Vec2::new_in_direction(dir_ns, 1);

                    let walk = map.get(pos);
                    let triv_ns = start.get(neighbor_ns);
                    let triv_ew = start.get(neighbor_ew);
                    let wall_ns = !map.get(neighbor_ns);
                    let wall_ew = !map.get(neighbor_ew);

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

pub fn build_trivial_navigation_graph<'a>(
    map: &Map,
    positions: &'a PositionBiMap,
) -> PosGraph<'a, 'a> {
    let mut graph = PosGraph::new(positions, positions);
    for (index_target, &pos) in positions.vec().iter().enumerate() {
        let trivials = get_trivial_navigables(map, pos);

        for start in trivials {
            let &index_start = positions.hashmap().get(&start).unwrap();
            graph.set(index_start, index_target, Some(distance(start, pos).into()));
        }
    }
    graph
}

pub fn find_beacons(max_beacon_dist: u32, trivial_navigation_graph: &PosGraph) -> Vec<usize> {
    let positions = *trivial_navigation_graph.get_map_start();
    let symmetric_graph = trivial_navigation_graph
        .symmetric_subgraph()
        .keep_edges(|_, _, dist| dist <= max_beacon_dist);

    // using symmetric graph to ensure traversability of beacon graph in all directions
    // using product of new reachable and connections to existing beacons to ensure
    // interconnectivity of beacon graph
    // Restricting max distance of each beacon, to have a dependable criterion on which beacons
    // don't have be checked when calculating possible entry and exit beacons.

    let mut unreached_positions = Graph::new_with_edge(positions, GraphMappingSingleNode, 0);
    let mut beacons = Graph::new(positions, GraphMappingSingleNode);
    let mut beacon_indices = Vec::<usize>::new();

    loop {
        let entry_exit = symmetric_graph.count_paths(&unreached_positions);
        let beacon_reachable = symmetric_graph.count_paths(&beacons);

        let value = if !beacon_indices.is_empty() {
            &entry_exit * &beacon_reachable
        } else {
            entry_exit.clone()
        };

        let (best_index, 0) = value.argmax().unwrap() else {
            panic!("something went wrong");
        };
        if value[(best_index, 0)] == 0 {
            panic!("Could not find interconnected beacon graph");
        }
        beacon_indices.push(best_index);

        beacons.set(best_index, 0, Some(0));
        unreached_positions = symmetric_graph
            .chain(&beacons)
            .map_edges(|x| x.is_none().then_some(0));

        if !unreached_positions.has_edges() {
            break;
        }
    }

    beacon_indices
}

pub fn get_beacon_info(
    beacon_indices: &[usize],
    positions: &PositionBiMap,
    graph: &PosGraph,
    beacon_graph: &PosGraph,
    max_beacon_dist: u32,
) -> BeaconInfo {
    let beacons = {
        let mut single_node = Graph::new(positions, GraphMappingSingleNode);
        for &index in beacon_indices {
            single_node.set(index, 0, Some(0));
        }
        single_node
    };

    BeaconInfo {
        max_beacon_dist,
        max_beacons_entry: *graph.count_paths(&beacons).max().unwrap(),
        max_beacons_exit: *beacons.invert_direction().count_paths(graph).max().unwrap(),
        max_path_length: u32::try_from(beacon_graph.all_pairs().1).unwrap(),
        n_beacons: u32::try_from(beacon_indices.len()).unwrap(),
    }
}
