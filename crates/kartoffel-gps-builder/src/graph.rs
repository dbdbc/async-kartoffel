use core::{clone::Clone, cmp::PartialEq, marker::PhantomData};

use kartoffel_gps::GlobalPos;
use ndarray::Array2;

use crate::map::PositionBiMap;

pub trait GraphMapping<T>: Clone {
    fn get_value(&self, index: usize) -> Option<T>;

    fn get_index(&self, t: T) -> Option<usize>;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // TODO why exactly did I not use Eq? I think there was a reason
    fn equals(&self, other: &Self) -> bool;

    fn index_mapping(&self, other: &impl GraphMapping<T>) -> Vec<usize> {
        (0..self.len())
            .map(|index| self.get_value(index).unwrap())
            .map(|value| {
                other
                    .get_index(value)
                    .expect("other GraphMapping should be a superset")
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct GraphMappingSingleNode;

impl GraphMapping<()> for GraphMappingSingleNode {
    fn get_value(&self, index: usize) -> Option<()> {
        (index == 0).then_some(())
    }

    fn get_index(&self, _t: ()) -> Option<usize> {
        Some(0)
    }

    fn len(&self) -> usize {
        1
    }

    fn equals(&self, _other: &Self) -> bool {
        true
    }
}

impl GraphMapping<GlobalPos> for &'_ PositionBiMap {
    fn get_value(&self, index: usize) -> Option<GlobalPos> {
        self.vec().get(index).copied()
    }

    fn get_index(&self, t: GlobalPos) -> Option<usize> {
        self.hashmap().get(&t).copied()
    }

    fn len(&self) -> usize {
        PositionBiMap::len(self)
    }

    fn equals(&self, other: &Self) -> bool {
        std::ptr::eq(*self, *other)
    }
}

#[derive(Debug)]
pub struct Graph<
    TStart,
    TDestination,
    MapStart: GraphMapping<TStart>,
    MapDestination: GraphMapping<TDestination>,
> {
    data: Array2<Option<u32>>,
    map_start: MapStart,
    map_destination: MapDestination,
    _phantom: PhantomData<(TStart, TDestination)>,
}

impl<
    TStart,
    TDestination,
    MapStart: GraphMapping<TStart>,
    MapDestination: GraphMapping<TDestination>,
> PartialEq for Graph<TStart, TDestination, MapStart, MapDestination>
{
    fn eq(&self, other: &Self) -> bool {
        self.map_start.equals(&other.map_start)
            && self.map_destination.equals(&other.map_destination)
            && self.data == other.data
    }
}

impl<
    TStart,
    TDestination,
    MapStart: GraphMapping<TStart>,
    MapDestination: GraphMapping<TDestination>,
> Clone for Graph<TStart, TDestination, MapStart, MapDestination>
{
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            map_start: self.map_start.clone(),
            map_destination: self.map_destination.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<
    TStart,
    TDestination,
    MapStart: GraphMapping<TStart>,
    MapDestination: GraphMapping<TDestination>,
> Graph<TStart, TDestination, MapStart, MapDestination>
{
    pub fn new(map_start: MapStart, map_destination: MapDestination) -> Self {
        let data = Array2::default((map_start.len(), map_destination.len()));
        Self {
            map_start,
            map_destination,
            data,
            _phantom: PhantomData,
        }
    }

    pub fn new_with_edge(map_start: MapStart, map_destination: MapDestination, val: u32) -> Self {
        let data = Array2::from_elem((map_start.len(), map_destination.len()), Some(val));
        Self {
            map_start,
            map_destination,
            data,
            _phantom: PhantomData,
        }
    }

    pub fn get_map_start(&self) -> &MapStart {
        &self.map_start
    }

    pub fn get_map_destination(&self) -> &MapDestination {
        &self.map_destination
    }

    pub fn len_start(&self) -> usize {
        self.map_start.len()
    }

    pub fn len_destination(&self) -> usize {
        self.map_destination.len()
    }

    pub fn get(&self, index_start: usize, index_destination: usize) -> Option<u32> {
        self.data[(index_start, index_destination)]
    }

    pub fn get_mut(&mut self, index_start: usize, index_destination: usize) -> &mut Option<u32> {
        &mut self.data[(index_start, index_destination)]
    }

    pub fn set(&mut self, index_start: usize, index_destination: usize, val: Option<u32>) {
        self.data[(index_start, index_destination)] = val;
    }

    pub fn count_paths<TNew, MapNew: GraphMapping<TNew>>(
        &self,
        other: &Graph<TDestination, TNew, MapDestination, MapNew>,
    ) -> Array2<u32> {
        assert!(self.map_destination.equals(&other.map_start));
        let mut n_paths = Array2::default((self.map_start.len(), other.map_destination.len()));
        for i_start in 0..self.len_start() {
            for i_new in 0..other.len_destination() {
                let mut count = 0;
                for i_middle in 0..self.len_destination() {
                    if self.get(i_start, i_middle).is_some() && other.get(i_middle, i_new).is_some()
                    {
                        count += 1;
                    }
                }
                n_paths[(i_start, i_new)] = count;
            }
        }
        n_paths
    }

    pub fn chain<TNew, MapNew: GraphMapping<TNew>>(
        &self,
        other: &Graph<TDestination, TNew, MapDestination, MapNew>,
    ) -> Graph<TStart, TNew, MapStart, MapNew> {
        assert!(self.map_destination.equals(&other.map_start));
        let mut new = Graph::new(self.map_start.clone(), other.map_destination.clone());
        let add_opts = |x1: Option<u32>, x2: Option<u32>| -> Option<u32> { Some(x1? + x2?) };
        let min_opts = |x1: Option<u32>, x2: Option<u32>| -> Option<u32> {
            match (x1, x2) {
                (Some(v1), Some(v2)) => Some(v1.min(v2)),
                (Some(v), None) | (None, Some(v)) => Some(v),
                _ => None,
            }
        };
        for i_start in 0..self.len_start() {
            for i_new in 0..other.len_destination() {
                let val = new.get_mut(i_start, i_new);
                for i_middle in 0..self.len_destination() {
                    *val = min_opts(
                        *val,
                        add_opts(self.get(i_start, i_middle), other.get(i_middle, i_new)),
                    );
                }
            }
        }
        new
    }

    pub fn sub_graph<MapStartNew: GraphMapping<TStart>, MapDestNew: GraphMapping<TDestination>>(
        &self,
        map_start: MapStartNew,
        map_destination: MapDestNew,
    ) -> Graph<TStart, TDestination, MapStartNew, MapDestNew> {
        let indices_start = map_start.index_mapping(&self.map_start);
        let indices_destination = map_destination.index_mapping(&self.map_destination);
        let mut ret = Graph::new(map_start, map_destination);
        for (i_start, &index_start) in indices_start.iter().enumerate() {
            for (i_dest, &index_dest) in indices_destination.iter().enumerate() {
                ret.set(i_start, i_dest, self.get(index_start, index_dest));
            }
        }
        ret
    }

    pub fn keep_edges(&self, f: impl Fn(TStart, TDestination, u32) -> bool) -> Self {
        let mut ret = self.clone();
        for i_start in 0..ret.len_start() {
            for i_dest in 0..ret.len_destination() {
                if let Some(weight) = ret.get(i_start, i_dest)
                    && !f(
                        ret.map_start
                            .get_value(i_start)
                            .expect("indices should be dense"),
                        ret.map_destination
                            .get_value(i_dest)
                            .expect("indices should be dense"),
                        weight,
                    )
                {
                    ret.remove_edge(i_start, i_dest);
                }
            }
        }
        ret
    }

    pub fn remove_edge(&mut self, index_start: usize, index_destination: usize) {
        self.set(index_start, index_destination, None);
    }

    pub fn invert_direction(&self) -> Graph<TDestination, TStart, MapDestination, MapStart> {
        Graph {
            data: self.data.t().to_owned(),
            map_start: self.map_destination.clone(),
            map_destination: self.map_start.clone(),
            _phantom: PhantomData,
        }
    }

    pub fn map_edges(&self, f: impl Fn(Option<u32>) -> Option<u32>) -> Self {
        Self {
            data: self.data.mapv(f),
            map_start: self.map_start.clone(),
            map_destination: self.map_destination.clone(),
            _phantom: PhantomData,
        }
    }

    pub fn has_edges(&self) -> bool {
        self.data.flatten().iter().any(|x| x.is_some())
    }

    pub fn fully_connected(&self) -> bool {
        self.data.flatten().iter().all(|x| x.is_some())
    }

    pub fn max_weight(&self) -> Option<u32> {
        self.data.flatten().iter().filter_map(|x| *x).max()
    }

    pub fn min_weight(&self) -> Option<u32> {
        self.data.flatten().iter().filter_map(|x| *x).min()
    }

    pub fn count_edges(&self) -> usize {
        self.data.flatten().iter().filter_map(|x| *x).count()
    }
}

impl<T, Mapping: GraphMapping<T>> Graph<T, T, Mapping, Mapping> {
    pub fn symmetric_subgraph(&self) -> Self {
        assert!(self.map_start.equals(&self.map_destination));
        let mut data = Array2::default((self.map_start.len(), self.map_start.len()));
        for i1 in 0..self.len_start() {
            for i2 in 0..self.len_destination() {
                if self.get(i1, i2) == self.get(i2, i1) {
                    data[(i1, i2)] = self.get(i1, i2);
                }
            }
        }
        Self {
            data,
            map_start: self.map_start.clone(),
            map_destination: self.map_destination.clone(),
            _phantom: PhantomData,
        }
    }

    pub fn all_pairs(&self) -> (Self, usize) {
        assert!(self.map_start.equals(&self.map_destination));
        let mut chained = self.clone();
        let mut i = 1usize;
        loop {
            let new = chained.chain(self);

            if new == chained {
                return (chained, i);
            } else {
                chained = new;
                i += 1;
            }
        }
    }
}
