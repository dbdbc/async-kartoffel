use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::ops::Div;
use std::path::Path;

use kartoffel_gps::gps::{MapSection, MapSectionTrait};
use kartoffel_gps::pos::pos_east_south;
use kartoffel_gps::GlobalPos;
use kartoffel_gps_builder::{
    beacon_nav::{build_trivial_navigation_graph, find_beacons, get_beacon_info},
    const_global_pos::ArrayBuilder,
    const_graph::ConstSparseGraphBuilder,
    map::Map,
};
use phf_codegen::Map as PhfMap;

fn main() {
    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("codegen.rs");
    let file = &mut BufWriter::new(File::create(&path).unwrap());

    let map_path = "maps/map-grotta.txt";
    println!("build.rs analysing map {}", map_path);
    println!("cargo::rerun-if-changed={}", map_path);
    let map = &Map::from_path(Path::new(map_path)).unwrap();

    add_true_map(file, map);
    add_gps::<MapSection<7>>(file, map);
    add_beacons(file, map, 5);
}

fn add_true_map(file: &mut BufWriter<impl Write>, map: &Map) {
    let builder = map.builder();
    writeln!(
        file,
        "const TRUE_MAP: {} = {};\n",
        builder.type_string(),
        builder
    )
    .unwrap();
}

fn add_beacons(file: &mut BufWriter<impl Write>, map: &Map, max_beacon_dist: u32) {
    println!("creating navigation beacons");
    println!("  finding walkable positions ...");
    let positions = map.walkable_positions();
    println!("  creating trivial navigation graph ...");
    let asymmetric_graph = build_trivial_navigation_graph(map, &positions);
    println!("  selecting navigation beacons ...");
    let beacon_indices = find_beacons(max_beacon_dist, &asymmetric_graph);
    println!("  create beacon trivial navigation (sub-)graph ...");
    let beacon_positions = asymmetric_graph.get_map_start().subset(&beacon_indices);
    let beacon_graph = asymmetric_graph.sub_graph(&beacon_positions, &beacon_positions);

    let beacon_info = get_beacon_info(
        &beacon_indices,
        &positions,
        &asymmetric_graph,
        &beacon_graph,
        max_beacon_dist,
    );
    println!("  beacon info: {:?}", beacon_info);

    let builder_graph = ConstSparseGraphBuilder::from_graph(&beacon_graph);
    let builder_pos = ArrayBuilder(beacon_positions.vec());
    writeln!(
        file,
        "const BEACON_GRAPH: {} = {};\n",
        builder_graph.type_string(),
        builder_graph
    )
    .unwrap();
    writeln!(
        file,
        "const BEACON_POSITIONS: {} = {};\n",
        builder_pos.type_string(),
        builder_pos
    )
    .unwrap();
    writeln!(
        file,
        "const BEACON_INFO: ::kartoffel_gps::beacon::BeaconInfo = {:?};\n",
        beacon_info
    )
    .unwrap();
    writeln!(
        file,
        "const NAV_MAX_PATH_LEN: usize = {};
const NAV_MAX_ENTRY_EXIT: usize = {};
const NAV_TRIV_BUFFER: usize = {};
const NAV_NODE_BUFFER: usize = {};
const NAV_ACTIVE_BUFFER: usize = {};
",
        (beacon_info.max_path_length * 2).next_power_of_two(), // multiply by two to hedge against (very unlikely) path updates
        beacon_info
            .max_beacons_entry
            .max(beacon_info.max_beacons_exit)
            .next_power_of_two(),
        (beacon_info.max_beacon_dist + 2).div(2),
        beacon_info.n_beacons + 2,
        (beacon_info.n_beacons + 2).next_power_of_two(),
    )
    .unwrap();

    let mut map_chars: Vec<Vec<char>> = Vec::new();
    for row in 0..map.height {
        let mut row_chars = Vec::new();
        for col in 0..map.width {
            let pos = pos_east_south(col as i16, -(row as i16));
            row_chars.push(if map.get(pos) { '.' } else { '#' });
        }
        map_chars.push(row_chars);
    }
    for pos in beacon_positions.vec() {
        let row = pos.subtract_anchor().south() as usize;
        let col = pos.subtract_anchor().east() as usize;
        map_chars[row][col] = '*';
    }
    writeln!(file, "// beacons").unwrap();
    for row in map_chars {
        writeln!(file, "// {}", row.into_iter().collect::<String>()).unwrap();
    }
    writeln!(file).unwrap();
}

fn add_gps<T: MapSectionTrait>(file: &mut BufWriter<impl Write>, map: &Map) {
    let (unique_chunks, n_total) = map.unique_chunks::<T>();
    {
        let n_unique = unique_chunks.len();
        let unique_percentage = n_unique as f64 / n_total as f64 * 100.0;
        println!(
            "writing unique chunks, {} of {} chunks are unique, that's {:.1}%",
            n_unique, n_total, unique_percentage
        );
    }

    let mut builder = PhfMap::new();
    for (chunk, center) in unique_chunks {
        let center_south = u8::try_from((center - GlobalPos::default()).south())
            .expect("center should be south (left)");
        let center_east = u8::try_from((center - GlobalPos::default()).east())
            .expect("center should be east (right)");
        builder.entry(
            chunk.compress().expect("center should be walkable"),
            &std::format!("({}, {})", center_east, center_south),
        );
    }

    writeln!(
        file,
        "static UNIQUE_CHUNKS: ::phf::Map<{}, (u8, u8)> = {};\n",
        <T>::compressed_type(),
        builder.build()
    )
    .unwrap();
}
