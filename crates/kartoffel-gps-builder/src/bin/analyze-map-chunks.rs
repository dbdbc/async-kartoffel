use std::{collections::HashMap, path::Path};

use kartoffel_gps::Chunk;
use kartoffel_gps::MapSection;
use kartoffel_gps_builder::Map;

fn main() {
    const CHUNK_SIZE: usize = 9;

    let map_location = Path::new("./maps/map.txt");
    let map = Map::from_path(map_location).unwrap();
    println!("walkable tiles: {}", map.n_walkable());

    let chunks = map.get_chunks::<Chunk<CHUNK_SIZE>>();

    let mut multiplicities: HashMap<usize, Vec<Chunk<CHUNK_SIZE>>> = HashMap::new();
    for (chunk, locations) in &chunks {
        let count = locations.len();
        multiplicities.entry(count).or_default().push(chunk.clone());
    }

    let mut counts = multiplicities.keys().collect::<Vec<_>>();
    counts.sort();

    for count in counts {
        let chunks = multiplicities.get(count).unwrap();
        println!("multiplicity {}: n_chunks {}", count, chunks.len());
        // for chunk in chunks {
        //     println!("{}", chunk);
        // }
    }

    println!("done");
}
