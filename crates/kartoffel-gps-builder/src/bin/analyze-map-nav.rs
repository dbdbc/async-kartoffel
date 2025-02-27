use std::path::Path;

use kartoffel_gps::const_graph::{ConstSparseGraph, ConstSparseGraphNode};
use kartoffel_gps_builder::{
    beacon_nav::{build_distance_graph, build_trivial_navigation_graph, find_beacons, sub_graph},
    const_global_pos::ArrayBuilder,
    const_graph::ConstSparseGraphBuilder,
    map::Map,
};
use ndarray::Array1;
use ndarray_stats::QuantileExt;

fn main() {
    let map = Map::from_path(Path::new("./maps/map.txt")).unwrap();

    let positions = map.walkable_positions();

    let n_walkable = positions.len();

    println!("walkable tiles: {}", n_walkable,);

    let asymmetric_graph = build_trivial_navigation_graph(&map, &positions);
    println!("trivial navigation graph built successfully");

    let dist_manhattan_graph = build_distance_graph(&positions);
    println!("manhattan distance graph built successfully");

    // The following calculation is only based on the graphs.
    // With graph matrix called A, initial one-hot position s, and target one-hot position t, the
    // number of combinations each position is reachable after exactly n steps is given by A^n s.

    let beacons = find_beacons(12, &asymmetric_graph, &dist_manhattan_graph);
    let n_beacons = beacons.len();
    println!(
        "greedy beacon algo finished, requires {} beacons",
        n_beacons
    );

    {
        let mut beacon_map = map.new_like();
        for &index in &beacons {
            beacon_map.set(positions.vec()[index], true).unwrap();
        }
        beacon_map
            .write_file(Path::new("./data/beacons.txt"))
            .unwrap();
    }

    let beacon_graph = sub_graph(&asymmetric_graph, &beacons);

    let beacon_positions = beacons
        .iter()
        .map(|&index| positions.vec()[index])
        .collect::<Vec<_>>();

    let beacon_graph_sym = &beacon_graph * &beacon_graph.t();

    {
        let to_pos = beacon_graph.dot(&Array1::ones(n_beacons));
        let from_pos = Array1::ones(n_beacons).dot(&beacon_graph);
        let both_pos = beacon_graph_sym.dot(&Array1::ones(n_beacons));
        for i in 0..n_beacons {
            println!(
                "beacon at: {:?}, from beacon to {} others, from {} others to beacon, {} both",
                positions.vec()[beacons[i]],
                from_pos[i],
                to_pos[i],
                both_pos[i],
            );
        }
    }

    {
        let mut a_pow = beacon_graph.clone();
        for i in 2..20 {
            a_pow = a_pow.dot(&beacon_graph);

            println!(
                "min combinations after {} steps: {}",
                i,
                a_pow.min().unwrap()
            );

            a_pow = a_pow.mapv(|x| if x == 0 { 0 } else { 1 });
        }
    }

    println!("done");

    let builder_graph = ConstSparseGraphBuilder::from_matrix(&beacon_graph);
    let builder_pos = ArrayBuilder(&beacon_positions);
    println!(
        "const BEACON_GRAPH: {} = {};",
        builder_graph.type_string(),
        builder_graph
    );
    println!(
        "const BEACON_POSITIONS: {} = {};",
        builder_pos.type_string(),
        builder_pos
    );

    // for i in 0..BEACON_GRAPH.size() {
    //     println!(
    //         "beacon at: {:?}, from beacon to {} others, from {} others to beacon",
    //         BEACON_POSITIONS[usize::from(i)],
    //         BEACON_GRAPH.after(i).len(),
    //         BEACON_GRAPH.before(i).len(),
    //     );
    // }
}
