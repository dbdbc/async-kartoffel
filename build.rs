use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() {
    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("codegen.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());

    println!("cargo::rerun-if-changed=maps/grotta/map.txt");
    println!("cargo::rerun-if-changed=maps/sandbox-tiny/map.txt");

    let map =
        kartoffel_gps_builder::Map::from_path(Path::new("./maps/sandbox-tiny/map.txt")).unwrap();
    let unique_chunks = map.unique_chunks::<7>();

    let mut builder = phf_codegen::Map::new();
    for (key, value) in unique_chunks {
        let center_south = u8::try_from(value.0).unwrap();
        let center_east = u8::try_from(value.1).unwrap();
        builder.entry(key, &std::format!("({}, {})", center_south, center_east));
    }

    write!(
        &mut file,
        "static UNIQUE_CHUNKS: phf::Map<::kartoffel_gps::Chunk<7>, (u8, u8)> = {}",
        builder.build()
    )
    .unwrap();
    write!(&mut file, ";\n").unwrap();
}
