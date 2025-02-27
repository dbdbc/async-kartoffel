use anyhow::anyhow;
use async_kartoffel::Tile;
use async_kartoffel::Vec2;
use core::default::Default;
use kartoffel_gps::gps::MapSection;
use kartoffel_gps::GlobalPos;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, LineWriter, Write},
    path::Path,
};

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

pub struct PositionBiMap {
    v: Vec<GlobalPos>,
    h: HashMap<GlobalPos, usize>,
}
impl PositionBiMap {
    pub fn vec(&self) -> &Vec<GlobalPos> {
        &self.v
    }
    pub fn hashmap(&self) -> &HashMap<GlobalPos, usize> {
        &self.h
    }
    pub fn len(&self) -> usize {
        self.v.len()
    }
}

impl Map {
    pub fn new_like(&self) -> Self {
        Self {
            tiles: vec![false; self.width * self.height],
            width: self.width,
            height: self.height,
        }
    }

    pub fn n_walkable(&self) -> usize {
        self.tiles.iter().filter(|&&b| b).count()
    }

    pub fn walkable_positions(&self) -> PositionBiMap {
        let mut positions_v: Vec<GlobalPos> = Default::default();
        let mut positions_h: HashMap<GlobalPos, usize> = Default::default();
        {
            let mut index = 0usize;
            for i_east in 0..self.width {
                for i_south in 0..self.height {
                    let pos = GlobalPos::default()
                        + Vec2::new_east(i_east as i16)
                        + Vec2::new_south(i_south as i16);
                    if self.get(pos).is_some_and(|b| b) {
                        positions_v.push(pos);
                        positions_h.insert(pos, index);
                        index += 1;
                    }
                }
            }
        }
        PositionBiMap {
            v: positions_v,
            h: positions_h,
        }
    }

    pub fn get(&self, pos: GlobalPos) -> Option<bool> {
        let vec = pos - GlobalPos::default();
        let south = usize::try_from(vec.south()).ok()?;
        let east = usize::try_from(vec.east()).ok()?;

        if south < self.height && east < self.width {
            Some(self.tiles[south * self.width + east])
        } else {
            None
        }
    }

    pub fn set(&mut self, pos: GlobalPos, val: bool) -> anyhow::Result<()> {
        let vec = pos - GlobalPos::default();
        let south = usize::try_from(vec.south())?;
        let east = usize::try_from(vec.east())?;

        if south < self.height && east < self.width {
            self.tiles[south * self.width + east] = val;
            Ok(())
        } else {
            Err(anyhow!("out of bounds"))
        }
    }

    /// get chunks where center is walkable
    pub fn get_chunks<T: MapSection>(&self) -> HashMap<T, Vec<GlobalPos>> {
        let mut chunks: HashMap<_, Vec<_>> = HashMap::new();
        for center_south in 0..i16::try_from(self.height).unwrap() {
            for center_east in 0..i16::try_from(self.width).unwrap() {
                let center = GlobalPos::default()
                    + Vec2::new_east(center_east)
                    + Vec2::new_south(center_south);
                let chunk = self.get_chunk::<T>(center);
                if chunk.at_center() {
                    chunks.entry(chunk).or_default().push(center);
                }
            }
        }
        chunks
    }

    pub fn unique_chunks<T: MapSection>(&self) -> Vec<(T, GlobalPos)> {
        let chunks = self.get_chunks::<T>();

        let mut unique_chunks = Vec::new();
        for (chunk, locations) in &chunks {
            if locations.len() == 1 {
                unique_chunks.push((chunk.clone(), locations[0]));
            }
        }
        unique_chunks
    }

    pub fn get_chunk<T: MapSection>(&self, center: GlobalPos) -> T {
        T::from_function(|vec| self.get(center + vec).is_some_and(|b| b))
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
            let walkable = Tile::from_char(char)
                .expect(&format!("encountered unkown char {}", char))
                .is_walkable_terrain();
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
                '↓' | '↑' | '→' | '←' => None,
                c => Some(
                    Tile::from_char(c)
                        .expect(&format!("encountered unknown char {}", c))
                        .is_walkable_terrain(),
                ),
            };
            vec.push(walkable);
        }
        Ok(())
    }
}
