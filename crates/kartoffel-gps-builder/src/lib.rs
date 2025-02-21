use anyhow::anyhow;
use kartoffel_gps::Chunk;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, LineWriter, Write},
    path::Path,
};

// TODO: include border as unwalkable when calculating chunks

#[derive(Debug)]
pub struct IncompleteMap {
    pub tiles: Vec<Option<bool>>,
    pub width: usize,
    pub height: usize,
}

#[derive(Debug)]
pub struct Map {
    pub tiles: Vec<bool>,
    pub width: usize,
    pub height: usize,
}

impl Map {
    pub fn n_walkable(&self) -> usize {
        self.tiles.iter().filter(|&&b| b).count()
    }

    pub fn get(&self, south: usize, east: usize) -> Option<bool> {
        if south < self.height && east < self.width {
            Some(self.tiles[south * self.width + east])
        } else {
            None
        }
    }

    pub fn get_chunks<const N: usize>(&self) -> HashMap<Chunk<N>, Vec<(usize, usize)>> {
        let mut chunks: HashMap<_, Vec<_>> = HashMap::new();
        for center_south in 0..self.height {
            for center_east in 0..self.width {
                let chunk = self.get_chunk(center_south, center_east);
                let center = (center_south, center_east);
                if chunk.center() {
                    chunks.entry(chunk).or_default().push(center);
                }
            }
        }
        chunks
    }

    pub fn unique_chunks<const N: usize>(&self) -> Vec<(Chunk<N>, (usize, usize))> {
        let chunks = self.get_chunks::<N>();

        let mut unique_chunks = Vec::new();
        for (chunk, locations) in &chunks {
            if locations.len() == 1 {
                unique_chunks.push((chunk.clone(), locations[0]));
            }
        }
        unique_chunks
    }

    pub fn get_chunk<const N: usize>(&self, center_south: usize, center_east: usize) -> Chunk<N> {
        let r = N / 2;
        assert!(N == r * 2 + 1);

        let mut chunk = Chunk::default();
        for i_south in 0..2 * r + 1 {
            for i_east in 0..2 * r + 1 {
                if center_south + i_south >= r
                    && center_east + i_east >= r
                    && center_south + i_south < self.height + r
                    && center_east + i_east < self.width + r
                {
                    chunk.0[i_south][i_east] = self
                        .get(center_south + i_south - r, center_east + i_east - r)
                        .unwrap();
                }
                // else leave default value (false)
            }
        }
        chunk
    }
    pub fn from_path(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let file = File::open(&path)?;
        let mut vec = Vec::new();
        let mut width = None;
        let mut height = 0usize;
        for (i, line) in BufReader::new(file).lines().enumerate() {
            let line = line?;
            let line_width = line.chars().count();
            if let Some(width) = width {
                if line_width != width {
                    return Err(anyhow!(
                        "{} line {}: length should be {} but was {}: {}",
                        path.as_ref().display(),
                        i,
                        width,
                        line_width,
                        line,
                    ));
                }
            } else {
                width = Some(line_width);
            }
            Self::process_line(&mut vec, &line)?;
            height += 1;
        }

        let width = width.ok_or(anyhow!("unknown line length -> zero lines?"))?;
        assert!(
            width * height == vec.len(),
            "width and heigth don't match vec len"
        );
        Ok(Self {
            tiles: vec,
            width,
            height,
        })
    }

    fn process_line(vec: &mut Vec<bool>, line: &str) -> anyhow::Result<()> {
        for char in line.chars() {
            let walkable = match char {
                '#' => false,
                '.' => true,
                c => return Err(anyhow!("encountered unknown char {}", c)),
            };
            vec.push(walkable);
        }
        Ok(())
    }

    pub fn write_file(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let file = File::create(path)?;
        let mut file = LineWriter::new(file);

        let mut first = true;
        for line in self.tiles.chunks(self.width) {
            if first {
                first = false;
            } else {
                write!(file, "{}", "\n")?;
            }
            for &walkable in line {
                write!(file, "{}", if walkable { "." } else { "#" })?;
            }
        }

        file.flush()?;

        Ok(())
    }

    pub fn from_imcomplete_maps(maps: &[IncompleteMap]) -> anyhow::Result<Self> {
        let first = maps.first().ok_or(anyhow!("should be at least one map"))?;
        if !maps.iter().all(|map| map.height == first.height) {
            return Err(anyhow!("heights should match"));
        }
        if !maps.iter().all(|map| map.width == first.width) {
            return Err(anyhow!("width should match"));
        }
        if !maps
            .iter()
            .all(|map| map.tiles.len() == map.height * map.width)
        {
            return Err(anyhow!("vector len in map does not match"));
        }
        let mut tile_iterators = maps.iter().map(|map| map.tiles.iter()).collect::<Vec<_>>();

        let mut tiles = Vec::new();
        tiles.reserve(first.tiles.len());
        let mut i = 0usize;
        while let Some(next) = tile_iterators
            .iter_mut()
            .filter_map(|iter| *iter.next()?)
            .try_fold(None, |combined: Option<bool>, this| {
                if let Some(all) = combined {
                    if this != all {
                        Err(anyhow!("conflict in map combination"))
                    } else {
                        Ok(Some(all))
                    }
                } else {
                    Ok(Some(this))
                }
            })?
        {
            tiles.push(next);
            i += 1;
        }
        if tiles.len() < first.height * first.width {
            Err(anyhow!(
                "not enough data, some tile is still unknown in line {} col {}",
                i.div_euclid(first.width) + 1,
                i.rem_euclid(first.width) + 1,
            ))
        } else {
            Ok(Self {
                tiles,
                height: first.height,
                width: first.width,
            })
        }
    }
}

impl IncompleteMap {
    pub fn from_path(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let file = File::open(&path)?;
        let mut vec = Vec::new();
        let mut width = None;
        let mut height = 0usize;
        for (i, line) in BufReader::new(file).lines().enumerate() {
            let line = line?;
            let line_width = line.chars().count();
            if let Some(width) = width {
                if line_width != width {
                    return Err(anyhow!(
                        "{} line {}: length should be {} but was {}: {}",
                        path.as_ref().display(),
                        i,
                        width,
                        line_width,
                        line,
                    ));
                }
            } else {
                width = Some(line_width);
            }
            Self::process_line(&mut vec, &line)?;
            height += 1;
        }

        let width = width.ok_or(anyhow!("unknown line length -> zero lines?"))?;
        assert!(
            width * height == vec.len(),
            "width and heigth don't match vec len"
        );
        Ok(Self {
            tiles: vec,
            width,
            height,
        })
    }

    fn process_line(vec: &mut Vec<Option<bool>>, line: &str) -> anyhow::Result<()> {
        for char in line.chars() {
            let walkable = match char {
                '#' => Some(false),
                '.' | '@' => Some(true),
                '↓' | '↑' | '→' | '←' => None,
                c => return Err(anyhow!("encountered unknown char {}", c)),
            };
            vec.push(walkable);
        }
        Ok(())
    }
}
