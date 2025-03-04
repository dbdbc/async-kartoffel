use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use kartoffel_gps::gps::{MapSection, MapSectionTrait};
use kartoffel_gps::GlobalPos;
use kartoffel_gps_builder::{
    beacon_nav::{build_trivial_navigation_graph, find_beacons},
    const_global_pos::ArrayBuilder,
    const_graph::ConstSparseGraphBuilder,
    map::Map,
};
use phf_codegen::Map as PhfMap;

fn main() {
    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("codegen.rs");
    let file = &mut BufWriter::new(File::create(&path).unwrap());

    let map_path = "maps/map.txt";
    println!("cargo::rerun-if-changed={}", map_path);
    let map = &Map::from_path(Path::new(map_path)).unwrap();

    add_true_map(file, map);
    add_gps::<MapSection<7>>(file, map);
    add_beacons(file, map, 12);
}

fn add_true_map(file: &mut BufWriter<impl Write>, map: &Map) {
    let builder = map.builder();
    write!(
        file,
        "const TRUE_MAP: {} = {};\n",
        builder.type_string(),
        builder
    )
    .unwrap();
}

fn add_beacons(file: &mut BufWriter<impl Write>, map: &Map, max_beacon_dist: u32) {
    let positions = map.walkable_positions();
    let asymmetric_graph = build_trivial_navigation_graph(&map, &positions);
    let beacon_indices = find_beacons(max_beacon_dist, &asymmetric_graph);
    let beacon_positions = asymmetric_graph.get_map_start().subset(&beacon_indices);
    let beacon_graph = asymmetric_graph.sub_graph(&beacon_positions, &beacon_positions);
    // let beacon_info = get_beacon_info(
    //     &beacon_indices,
    //     positions.vec(),
    //     &asymmetric_graph,
    //     &beacon_graph,
    //     &beacon_positions,
    //     max_beacon_dist,
    // );

    let builder_graph = ConstSparseGraphBuilder::from_graph(&beacon_graph);
    let builder_pos = ArrayBuilder(beacon_positions.vec());
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

fn add_gps<T: MapSectionTrait>(file: &mut BufWriter<impl Write>, map: &Map) {
    let unique_chunks = map.unique_chunks::<T>();

    let mut builder = PhfMap::new();
    for (chunk, center) in unique_chunks {
        let center_south = u8::try_from((center - GlobalPos::default()).south())
            .expect("center should be south (left)");
        let center_east = u8::try_from((center - GlobalPos::default()).east())
            .expect("center should be east (right)");
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
