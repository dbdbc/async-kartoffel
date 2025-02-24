use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use async_kartoffel::{Direction, Global, Vec2};
use kartoffel_gps_builder::Map;
use ndarray::{Array1, Array2};
use ndarray_stats::QuantileExt;

fn get_trivial_navigables(map: &Map, target: Vec2<Global>) -> Vec<Vec2<Global>> {
    let mut start = map.new_like();

    let mut ret: Vec<Vec2<Global>> = Default::default();

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

    // TODO
    // start
    //     .write_file(Path::new(&format!(
    //         "./data/map_debug_{}_{}.txt",
    //         target.east(),
    //         target.south()
    //     )))
    //     .unwrap();

    ret
}

fn main() {
    let map = Map::from_path(Path::new("./maps/map.txt")).unwrap();

    // for both directions: index -> pos and pos -> index
    let mut positions_v: Vec<Vec2<Global>> = Default::default();
    let mut positions_h: HashMap<Vec2<Global>, usize> = Default::default();
    {
        let mut index = 0usize;
        for i_east in 0..map.width {
            for i_south in 0..map.height {
                let pos = Vec2::new_east(i_east as i16) + Vec2::new_south(i_south as i16);
                if map.get(pos).is_some_and(|b| b) {
                    positions_v.push(pos);
                    positions_h.insert(pos, index);
                    index += 1;
                }
            }
        }
    }

    assert!(
        positions_v.len() == positions_h.len(),
        "error, lens didn't match"
    );
    let n_walkable = positions_v.len();

    println!("walkable tiles: {}", n_walkable,);

    let mut trivial_nav_graph = Array2::<u32>::zeros((n_walkable, n_walkable));

    for (index_target, &pos) in positions_v.iter().enumerate() {
        let trivials = get_trivial_navigables(&map, pos);
        println!(
            "pos: {:?}, trivial starting positions: {}",
            pos,
            trivials.len()
        );

        for start in trivials {
            let index_start = positions_h.get(&start).unwrap();
            trivial_nav_graph[(index_target, *index_start)] = 1;
        }
    }

    println!("graph built successfully");
    // The following calculation is only based on the graph.
    // With graph matrix called A, initial one-hot position s, and target one-hot position t, the
    // number of combinations each position is reachable after exactly n steps is given by A^n s.

    let mat = trivial_nav_graph.mapv(|x| if x == 0 { 0 } else { 1u32 });

    let to_pos = mat.dot(&Array1::ones(n_walkable));
    let from_pos = Array1::ones(n_walkable).dot(&mat);
    for i in 0..n_walkable {
        println!(
            "pos: {:?}, from pos to {} others, from {} others to pos",
            positions_v[i], from_pos[i], to_pos[i]
        );
    }

    let beacons = {
        let mut a = Array1::ones(n_walkable);
        let mut b = Array1::ones(n_walkable);

        let mut beacons = Vec::<usize>::new();
        let mut beacon_arr = Array1::zeros(n_walkable);
        // entries and exits
        loop {
            let exits = a.dot(&mat); // How many new positions can be reached from i?
            let entries = mat.dot(&b); // i can be reached from how many new positions?

            let value = exits + entries;

            let best_index = value.argmax().unwrap();
            let best_val = value[best_index];
            println!(
                "best index {}: pos {:?} with value {}",
                best_index, positions_v[best_index], best_val
            );
            beacons.push(best_index);

            let mut best_one_hot = Array1::zeros(n_walkable);
            best_one_hot[best_index] = 1;
            a = a * (1 - mat.dot(&best_one_hot));
            b = b * (1 - best_one_hot.dot(&mat));

            beacon_arr = 1 - (1 - beacon_arr) * (1 - best_one_hot);

            if a.iter().all(|&x| x == 0) && b.iter().all(|&x| x == 0) {
                break;
            }
        }

        println!("interconnections");

        // beacon to beacon interconnections
        loop {
            let n_beacons = beacons.len();
            let mut beacon_graph = Array2::zeros((n_beacons, n_beacons));
            for i1 in 0..n_beacons {
                for i2 in 0..n_beacons {
                    beacon_graph[(i1, i2)] = mat[(beacons[i1], beacons[i2])];
                }
            }

            let n_connections1 = beacon_graph.dot(&Array1::ones(n_beacons));
            let n_connections2 = Array1::ones(n_beacons).dot(&beacon_graph);

            let connections_required = 3;
            // beacons with too few connections
            let critical = 1u32
                - n_connections1.mapv(|x| if x < connections_required { 0u32 } else { 1 })
                    * n_connections2.mapv(|x| if x < connections_required { 0u32 } else { 1 });
            let mut critical_large = Array1::zeros(n_walkable);
            let mut should_break = true;
            for (index_beacon, &val) in critical.iter().enumerate() {
                if val > 0u32 {
                    critical_large[beacons[index_beacon]] = 1;
                    should_break = false;
                }
            }
            if should_break {
                break;
            }

            let exits = critical_large.dot(&mat); // How many critical beacons can be reached from i?
            let entries = mat.dot(&critical_large); // i can be reached from how many critical beacons?
            let value = exits + entries;

            let best_index = value.argmax().unwrap();
            let best_val = value[best_index];
            println!(
                "best index {}: pos {:?} with value {}",
                best_index, positions_v[best_index], best_val
            );
            beacons.push(best_index);

            let mut best_one_hot = Array1::zeros(n_walkable);
            best_one_hot[best_index] = 1;
            beacon_arr = 1u32 - (1u32 - beacon_arr) * (1u32 - best_one_hot);
        }
        beacons
    };
    let n_beacons = beacons.len();
    println!(
        "greedy beacon algo finished, requires {} beacons",
        n_beacons
    );

    {
        let mut beacon_map = map.new_like();
        for &index in &beacons {
            beacon_map.set(positions_v[index], true).unwrap();
        }
        beacon_map
            .write_file(Path::new("./data/beacons.txt"))
            .unwrap();
    }

    let mut beacon_graph = Array2::zeros((n_beacons, n_beacons));
    for i1 in 0..n_beacons {
        for i2 in 0..n_beacons {
            beacon_graph[(i1, i2)] = mat[(beacons[i1], beacons[i2])];
        }
    }
    println!("{:?}", beacon_graph.dot(&Array1::ones(n_beacons)));
    println!("{:?}", Array1::ones(n_beacons).dot(&beacon_graph));

    let mut a_pow = beacon_graph.clone();
    for i in 2..50 {
        a_pow = a_pow.dot(&beacon_graph);

        println!(
            "min combinations after {} steps: {}",
            i,
            a_pow.min().unwrap()
        );

        a_pow = a_pow.mapv(|x| if x == 0 { 0 } else { 1 });
    }

    println!("done");
}
