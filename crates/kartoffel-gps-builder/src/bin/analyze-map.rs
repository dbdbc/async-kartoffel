use std::{collections::HashMap, path::Path};

use kartoffel_gps::{Chunk, Map};

fn main() {
    const CHUNK_SIZE: usize = 9;

    let map_dir = Path::new("./data/maps/grotta");
    let map = Map::from_path(map_dir.join("map.txt")).unwrap();
    println!("walkable tiles: {}", map.n_walkable());

    let mut chunks: HashMap<Chunk<CHUNK_SIZE>, Vec<(usize, usize)>> = HashMap::new();
    let (range_south, range_east) = Chunk::<CHUNK_SIZE>::ranges(map.height, map.width);
    for center_south in range_south.clone() {
        for center_east in range_east.clone() {
            let chunk = map.get_chunk(center_south, center_east).unwrap();
            let center = (center_south, center_east);
            if chunk.center() {
                chunks.entry(chunk).or_default().push(center);
            }
        }
    }

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
