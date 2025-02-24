use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use kartoffel_gps::Chunk;
use kartoffel_gps::MapSection;

fn main() {
    add_gps::<Chunk<9>>();
}

fn add_gps<T: MapSection>() {
    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("codegen.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());

    let map_path = "maps/map.txt";

    println!("cargo::rerun-if-changed={}", map_path);

    let map = kartoffel_gps_builder::Map::from_path(Path::new(map_path)).unwrap();
    let unique_chunks = map.unique_chunks::<T>();

    let mut builder = phf_codegen::Map::new();
    for (chunk, center) in unique_chunks {
        let center_south = u8::try_from(center.south()).expect("center should be south (left)");
        let center_east = u8::try_from(center.east()).expect("center should be east (right)");
        builder.entry(
            chunk.compress().expect("center should be walkable"),
            &std::format!("({}, {})", center_south, center_east),
        );
    }

    write!(
        &mut file,
        "static UNIQUE_CHUNKS: phf::Map<{}, (u8, u8)> = {}",
        <T>::compressed_type(),
        builder.build()
    )
    .unwrap();
    write!(&mut file, ";\n").unwrap();
}
