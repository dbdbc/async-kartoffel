use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use kartoffel_gps::gps::{Chunk, MapSection};
use kartoffel_gps_builder::{
    beacon_nav::{build_distance_graph, build_trivial_navigation_graph, find_beacons, sub_graph},
    const_graph::ConstSparseGraphBuilder,
    const_vec2::ArrayBuilder,
    map::Map,
};
use phf_codegen::Map as PhfMap;

fn main() {
    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("codegen.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());

    let map_path = "maps/map.txt";
    println!("cargo::rerun-if-changed={}", map_path);
    let map = Map::from_path(Path::new(map_path)).unwrap();

    add_gps::<Chunk<7>>(&mut file, &map);
    add_beacons(&mut file, &map, 12);
}

fn add_beacons(file: &mut BufWriter<impl Write>, map: &Map, max_beacon_dist: u32) {
    let positions = map.walkable_positions();
    let asymmetric_graph = build_trivial_navigation_graph(&map, &positions);
    let distance_graph = build_distance_graph(&positions);
    let beacon_indices = find_beacons(max_beacon_dist, &asymmetric_graph, &distance_graph);
    let beacon_graph = sub_graph(&asymmetric_graph, &beacon_indices);
    let beacon_positions = beacon_indices
        .iter()
        .map(|&index| positions.vec()[index])
        .collect::<Vec<_>>();

    let builder_graph = ConstSparseGraphBuilder::from_matrix(&beacon_graph);
    let builder_pos = ArrayBuilder(&beacon_positions);
    write!(
        file,
        "const BEACON_GRAPH: {} = {};\n",
        builder_graph.type_string(),
        builder_graph
    )
    .unwrap();
    write!(
        file,
        "const BEACON_POSITIONS: {} = {};\n",
        builder_pos.type_string(),
        builder_pos
    )
    .unwrap();
}

fn add_gps<T: MapSection>(file: &mut BufWriter<impl Write>, map: &Map) {
    let unique_chunks = map.unique_chunks::<T>();

    let mut builder = PhfMap::new();
    for (chunk, center) in unique_chunks {
        let center_south = u8::try_from(center.south()).expect("center should be south (left)");
        let center_east = u8::try_from(center.east()).expect("center should be east (right)");
        builder.entry(
            chunk.compress().expect("center should be walkable"),
            &std::format!("({}, {})", center_south, center_east),
        );
    }

    write!(
        file,
        "static UNIQUE_CHUNKS: ::phf::Map<{}, (u8, u8)> = {};\n",
        <T>::compressed_type(),
        builder.build()
    )
    .unwrap();
}
