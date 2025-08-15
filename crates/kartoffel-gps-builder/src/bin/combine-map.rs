use std::path::Path;

use kartoffel_gps_builder::map::{IncompleteMap, Map};

fn main() {
    let map_dir = Path::new("./data/maps/grotta");
    let incomplete_dir = map_dir.join("incomplete");

    let incomplete_files = std::fs::read_dir(incomplete_dir)
        .unwrap()
        .map(|path| path.unwrap().path())
        .collect::<Vec<_>>();
    let incomplete_maps = incomplete_files
        .iter()
        .map(|file| IncompleteMap::from_path(file).unwrap())
        .collect::<Vec<_>>();

    let map = Map::from_imcomplete_maps(&incomplete_maps).unwrap();

    map.write_file(map_dir.join("map.txt")).unwrap();

    println!("done");
}
