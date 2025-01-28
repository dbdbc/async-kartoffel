use core::{convert::identity, fmt::Display, marker::PhantomData};

use heapless::{FnvIndexSet, Vec};
use kartoffel::println;

use crate::{algorithm::breakpoint::Breakpoint, Distance, Error, Position, RadarSize};

use super::{terrain::Terrain, Map};

#[derive(Debug)]
struct Progress<const N: usize> {
    /// Positions that may be reachable, but have not yet been checked for walkability.
    active: Vec<Position, N>,
    /// Positions who have been checked for walkablility, and check has returned unknown. Those
    /// positions can be converted into active ones at a later stage.
    stale: FnvIndexSet<Position, N>,
}

#[derive(Debug)]
pub enum State<T> {
    Ready,
    Running(T),
    Halted(T),
    Completed,
    Error,
}

impl<T> State<T> {
    fn strip_data(&self) -> State<()> {
        match self {
            Self::Ready => State::Ready,
            Self::Running(_) => State::Running(()),
            Self::Halted(_) => State::Halted(()),
            Self::Completed => State::Completed,
            Self::Error => State::Error,
        }
    }
}

impl Display for State<()> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            State::Ready => write!(f, "{:?}", self),
            State::Running(_) => write!(f, "Running"),
            State::Halted(_) => write!(f, "Halted"),
            State::Completed => write!(f, "{:?}", self),
            State::Error => write!(f, "{:?}", self),
        }
    }
}

impl<const N: usize> State<Progress<N>> {
    fn with(&mut self, f: impl FnOnce(Self) -> Self) {
        // invalidate self, take ownership, perform mutation and set self again
        *self = f(core::mem::replace(self, Self::Error));
    }
    fn activate(&mut self) {
        self.with(|state| {
            if let Self::Halted(progress) = state {
                Self::Running(progress)
            } else {
                state
            }
        });
    }
    fn halt(&mut self) {
        self.with(|state| {
            if let Self::Running(progress) = state {
                Self::Halted(progress)
            } else {
                state
            }
        });
    }

    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Completed)
    }
}

/// A interruptable computation to keep track of which positions can be reached. The actual data
/// has to be supplied as function argument. This allows this to be an optional extension to
/// Map<Terrain>. Could also be implemented as a wrapper, with integrated activation on radar scan.
/// TODO I'm not sure about this API yet.
pub struct Exploration<const N: usize, T: Map<Terrain>> {
    state: State<Progress<N>>,
    _phantom: PhantomData<T>,
}

impl<const N: usize, T: Map<Terrain>> Default for Exploration<N, T> {
    fn default() -> Self {
        assert!(N >= 1);
        Self {
            state: State::Ready,
            _phantom: PhantomData,
        }
    }
}

fn get_terrain(map: &impl Map<Terrain>, pos: Position) -> Terrain {
    map.get(pos).unwrap_or(Terrain::Unknown)
}

fn set_reachable(map: &mut impl Map<Terrain>, pos: Position, reachable: bool) -> Result<(), Error> {
    match map.get(pos) {
        Some(Terrain::Unknown) => panic!(),
        Some(Terrain::Blocked) => {
            if reachable {
                Err(Error::Inconsistent)
            } else {
                Ok(())
            }
        }
        Some(Terrain::Walkable) | Some(Terrain::Reachable) => {
            if reachable {
                map.set(pos, Terrain::Reachable)
                    .map_err(|_| Error::OutOfMemory)?;
                Ok(())
            } else {
                // TODO represent unreachable but walkable?
                Err(Error::Inconsistent)
            }
        }
        None => panic!(),
    }
}

impl<const N: usize, T: Map<Terrain>> Exploration<N, T> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn initialize(&mut self, map: &mut T, initial: Position) {
        assert!(get_terrain(map, initial).is_known_walkable());
        set_reachable(map, initial, true).unwrap();
        self.state = State::Running(Progress {
            active: Vec::from_slice(&[initial]).unwrap(),
            stale: Default::default(),
        });
    }

    pub fn get_state(&self) -> State<()> {
        self.state.strip_data()
    }

    /// cancelable async function that runs until there are no more active positions
    pub async fn run(&mut self, map: &mut T) {
        fn inner<T: Map<Terrain>, const N: usize>(
            progress: &mut Progress<N>,
            map: &mut T,
            pos: Position,
        ) -> Result<(), Error> {
            match get_terrain(map, pos).is_walkable() {
                Some(walkable) => {
                    set_reachable(map, pos, walkable).map_err(|_| Error::OutOfMemory)?;
                    if walkable {
                        for (neighbor, _) in pos.neighbors() {
                            if get_terrain(map, neighbor).is_reachable().is_none() {
                                progress
                                    .active
                                    .push(neighbor)
                                    .map_err(|_| Error::OutOfMemory)?;
                            }
                        }
                    }
                }
                None => _ = progress.stale.insert(pos).map_err(|_| Error::OutOfMemory)?,
            }
            Ok(())
        }
        if let State::Running(progress) = &mut self.state {
            while let Some(pos) = progress.active.pop() {
                if inner(progress, map, pos).is_err() {
                    self.state = State::Error;
                    return;
                };
                // Future can be dropped at this point without leaving self in an invalid state
                Breakpoint::new().await;
            }
            if progress.active.is_empty() {
                if progress.stale.is_empty() {
                    self.state = State::Completed;
                } else {
                    println!("halting, stale: {}", progress.stale.iter().count());
                    self.state.halt();
                }
            }
        }
    }

    pub fn activate<Size: RadarSize>(&mut self, center: Position) -> Result<(), Error> {
        match &mut self.state {
            State::Ready => (),
            State::Running(progress) => {
                for i_east in Size::range() {
                    for i_north in Size::range() {
                        let pos = center + Distance::new_global(i_east.into(), i_north.into());
                        if progress.stale.remove(&pos) {
                            progress.active.push(pos).map_err(|_| Error::OutOfMemory)?;
                        }
                    }
                }
            }
            State::Halted(progress) => {
                for i_east in Size::range() {
                    for i_north in Size::range() {
                        let pos = center + Distance::new_global(i_east.into(), i_north.into());
                        if progress.stale.remove(&pos) {
                            progress.active.push(pos).map_err(|_| Error::OutOfMemory)?;
                        }
                    }
                }
            }
            State::Completed => (),
            State::Error => (),
        }
        self.state.activate();
        Ok(())
    }

    /// reachable positions with adjacent unknowns
    pub fn border<'t, 's>(
        &'s self,
        map: &'t T,
    ) -> Option<impl Iterator<Item = Position> + use<'s, 't, T, N>> {
        let filter = |(pos, _)| {
            get_terrain(map, pos)
                .is_reachable()
                .is_some_and(identity)
                .then_some(pos)
        };
        match &self.state {
            State::Ready => None,
            State::Running(progress) => Some(
                progress
                    .stale
                    .iter()
                    .cloned()
                    .flat_map(Position::neighbors)
                    .filter_map(filter),
            ),
            State::Halted(progress) => Some(
                progress
                    .stale
                    .iter()
                    .cloned()
                    .flat_map(Position::neighbors)
                    .filter_map(filter),
            ),
            State::Completed => None,
            State::Error => None,
        }
    }
}
