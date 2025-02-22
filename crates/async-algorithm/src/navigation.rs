use core::num::NonZeroU16;

use async_kartoffel::{Direction, Position};
use heapless::{FnvIndexMap, Vec};

use super::{
    breakpoint::Breakpoint,
    error::{NoTarget, OutOfMemory},
    DistanceManhattan, DistanceMeasure, Map,
};

// possibly implemented as bitfield
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct DirectionCombination {
    pub north: bool,
    pub east: bool,
    pub south: bool,
    pub west: bool,
}

impl DirectionCombination {
    pub fn any(&self) -> bool {
        self.north || self.east || self.south || self.west
    }
    pub fn get(&self, dir: Direction) -> bool {
        match dir {
            Direction::East => self.east,
            Direction::North => self.north,
            Direction::West => self.west,
            Direction::South => self.south,
        }
    }
    pub fn set(&mut self, dir: Direction, val: bool) {
        match dir {
            Direction::East => self.east = val,
            Direction::North => self.north = val,
            Direction::West => self.west = val,
            Direction::South => self.south = val,
        }
    }
    pub fn all(&self) -> Vec<Direction, 4> {
        let mut ret = Vec::new();
        for dir in Direction::all() {
            if self.get(dir) {
                // unwrap: there can't be more than four elements
                ret.push(dir).unwrap();
            }
        }
        ret
    }
}

#[derive(Debug)]
struct Progress<const N: usize> {
    /// Active positions with cost [`Self::cost_current`]
    active_current: Vec<Position, N>,

    /// Active positions with cost larger than [`Self::cost_current`]
    active_next: FnvIndexMap<Position, u16, N>,

    /// The cost for every item in [`Self::active_current`]. For a given active position, cost is
    /// calculated as distance_manhattan between the position and [NavigationTask::from], plus
    /// distance (in tiles to travel) between the position and [NavigationTask::to], therefore cost
    /// is the minimum possible travel distance between from and to according to current knowledge.
    cost_current: u16,

    task: NavigationTask,
}

enum NavigationResult {
    Impossible,
    Success,
}

impl<const N: usize> Progress<N> {
    /// if active_current is empty, refill is with minimal cost items from active_next
    fn set_new_current_cost(&mut self, cost_minimum: u16) {
        assert!(self.active_current.is_empty());
        self.cost_current = cost_minimum;

        // move items with cost_minimum from active_next to active_current
        for (&pos, &cost) in &self.active_next {
            assert!(cost >= cost_minimum);
            if cost == cost_minimum {
                // unwrap: same size for both vecs
                self.active_current.push(pos).unwrap();
            }
        }
        self.active_next
            .retain(|_pos, &mut cost| cost > cost_minimum);
    }

    // TODO make interruptable
    fn change_start(&mut self, new_start: Position) -> Result<(), OutOfMemory> {
        if new_start == self.task.from {
            return Ok(());
        }

        let old_start = self.task.from;
        self.task = NavigationTask {
            from: new_start,
            to: self.task.to,
        };
        let new_cost = |old_cost: u16, pos: Position| {
            let dist = old_cost
                .checked_sub(DistanceManhattan::measure(pos - old_start))
                .unwrap();
            dist + DistanceManhattan::measure(pos - new_start)
        };

        // update all costs, push everything into active_next, so active_current is empty
        for (&pos, cost) in self.active_next.iter_mut() {
            *cost = new_cost(*cost, pos);
        }
        while let Some(pos) = self.active_current.pop() {
            self.active_next
                .insert(pos, new_cost(self.cost_current, pos))
                .map_err(|_| OutOfMemory)?;
        }

        if let Some(cost_minimum) = self.active_next.iter().map(|(_pos, &cost)| cost).min() {
            self.set_new_current_cost(cost_minimum);
        }
        Ok(())
    }

    async fn run(
        &mut self,
        distances: &mut impl Map<Option<NonZeroU16>>,
        can_go: impl Fn(Position) -> bool,
    ) -> Result<NavigationResult, OutOfMemory> {
        if !can_go(self.task.to) {
            Ok(NavigationResult::Impossible)
        } else {
            loop {
                while let Some(pos) = self.active_current.pop() {
                    // unwrap: must be positive due to definition
                    let distance_current = self
                        .cost_current
                        .checked_sub(DistanceManhattan::measure(pos - self.task.from))
                        .unwrap();
                    for (neighbor, _) in pos.neighbors() {
                        let distance_neighbor = distance_current + 1;
                        if neighbor == self.task.from {
                            distances_set(distances, neighbor, distance_neighbor)?;
                            return Ok(NavigationResult::Success);
                        }
                        let dist_neighbor_prev = distances_get(distances, neighbor);
                        if can_go(neighbor)
                            && dist_neighbor_prev
                                .is_none_or(|dist_prev| dist_prev > distance_neighbor)
                        {
                            distances_set(distances, neighbor, distance_neighbor)?;
                            let cost_neighbor = distance_neighbor
                                + DistanceManhattan::measure(neighbor - self.task.from);
                            match cost_neighbor.cmp(&self.cost_current) {
                                core::cmp::Ordering::Less => unreachable!(),
                                core::cmp::Ordering::Equal => {
                                    self.active_current
                                        .push(neighbor)
                                        .map_err(|_| OutOfMemory)?;
                                    // Required in case dist_neighbor_prev existed.
                                    // Double push of the same position into active_current is
                                    // not possible, because we require an improvement to cost
                                    // (distance_neighbor < dist_prev)
                                    self.active_next.remove(&neighbor);
                                }
                                core::cmp::Ordering::Greater => {
                                    self.active_next
                                        .insert(neighbor, cost_neighbor)
                                        .map_err(|_| OutOfMemory)?;
                                }
                            }
                        }
                    }
                    // future is cancellable here
                    Breakpoint::new().await;
                }

                // active_current is now empty

                if self.active_next.is_empty() {
                    return Ok(NavigationResult::Impossible);
                }

                // Increase by 2, because cost is either odd or even, but the same, for all paths
                self.set_new_current_cost(self.cost_current + 2);
            }
        }
    }
}

#[derive(Debug)]
pub enum State<T> {
    Ready,
    Running(T),
    Success(T),
    Error(OutOfMemory),
    Impossible(T),
}
impl<const N: usize> State<Progress<N>> {
    fn strip_data(&self) -> State<NavigationTask> {
        match self {
            State::Ready => State::Ready,
            State::Running(progress) => State::Running(progress.task),
            State::Success(progress) => State::Success(progress.task),
            State::Error(err) => State::Error(*err),
            State::Impossible(progress) => State::Impossible(progress.task),
        }
    }
}
impl State<NavigationTask> {
    pub fn task(&self) -> Option<NavigationTask> {
        match self {
            State::Ready => None,
            State::Running(task) => Some(*task),
            State::Success(task) => Some(*task),
            State::Error(_) => None,
            State::Impossible(task) => Some(*task),
        }
    }
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }
}

impl<const N: usize> State<Progress<N>> {
    fn with(&mut self, f: impl FnOnce(Self) -> Self) {
        // This seems to optimize the stack allocation away for simple functions, but is it not
        // guaranteed. Invalidate self, take ownership, perform mutation and set self again.
        *self = f(core::mem::replace(self, Self::Error(OutOfMemory)));
    }
    /// change state to [`State::Success`] without heap allocations
    fn success(&mut self) {
        self.with(|state| match state {
            State::Running(progress) => State::Success(progress),
            State::Impossible(progress) => State::Success(progress),
            _ => state,
        });
    }
    /// change state to [`State::Running`] without heap allocations
    fn running(&mut self) {
        self.with(|state| match state {
            State::Success(progress) => State::Running(progress),
            State::Impossible(progress) => State::Running(progress),
            _ => state,
        });
    }
    /// change state to [`State::Impossible`] without heap allocations
    fn impossible(&mut self) {
        self.with(|state| match state {
            State::Success(progress) => State::Impossible(progress),
            State::Running(progress) => State::Impossible(progress),
            _ => state,
        });
    }
    fn error(&mut self, err: OutOfMemory) {
        *self = State::Error(err);
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub struct NavigationTask {
    pub from: Position,
    pub to: Position,
}

impl core::fmt::Display for NavigationTask {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} -> {}", self.from, self.to)
    }
}

/// A interruptable navigation computation to a fixed target.
pub struct Navigation<T: Map<Option<NonZeroU16>>, const N: usize> {
    /// Actually does not store distance, but distance plus one, to take advantage of niche
    /// optimizations. It should only be modified using [`distances_set`] to prevent errors.
    distances: T,
    state: State<Progress<N>>,
}

impl<T: Map<Option<NonZeroU16>> + Default, const N: usize> Default for Navigation<T, N> {
    fn default() -> Self {
        Self {
            distances: Default::default(),
            state: State::Ready,
        }
    }
}

/// handles Option<NonZeroU16> and addition of 1
fn distances_set(
    distances: &mut impl Map<Option<NonZeroU16>>,
    pos: Position,
    distance: u16,
) -> Result<(), OutOfMemory> {
    distances
        .set(pos, NonZeroU16::new(1 + distance))
        .map_err(|_| OutOfMemory)
}
fn distances_get(distances: &impl Map<Option<NonZeroU16>>, pos: Position) -> Option<u16> {
    match distances.get(pos) {
        Some(Some(dist)) => Some(u16::from(dist) - 1),
        _ => None,
    }
}

impl<T: Map<Option<NonZeroU16>>, const N: usize> Navigation<T, N> {
    pub fn new(distances: T) -> Self {
        assert!(N >= 1);
        Self {
            distances,
            state: State::Ready,
        }
    }
    pub fn initialize(&mut self, from: Position, to: Position) {
        // Initialize active_current as default to prevent allocating it on the stack. Not pretty,
        // but prevents stack overflow.
        self.state = State::Running(Progress {
            active_current: Default::default(),
            active_next: Default::default(),
            cost_current: DistanceManhattan::measure(from - to),
            task: NavigationTask { from, to },
        });
        let State::Running(ref mut progress) = self.state else {
            panic!()
        };
        // unwrap: we checked N >= 1
        progress.active_current.push(to).unwrap();
        if from == to {
            self.state.success();
        };
        self.distances.clear();
        distances_set(&mut self.distances, to, 0)
            .expect("map should always allow the initial entry to succeed");
    }

    pub fn get_state(&self) -> State<NavigationTask> {
        self.state.strip_data()
    }

    // TODO implement lazy, and add actual computation to run
    pub fn update_start(&mut self, new_start: Position) -> Result<(), NoTarget> {
        enum StateWithoutData {
            Success,
            Running,
            Error(OutOfMemory),
        }

        fn update_task<T: Map<Option<NonZeroU16>>, const N: usize>(
            distances: &mut T,
            progress: &mut Progress<N>,
            new_start: Position,
        ) -> StateWithoutData {
            match progress.change_start(new_start) {
                Ok(()) => {
                    if let Some(Some(_)) = distances.get(new_start) {
                        StateWithoutData::Success
                    } else {
                        StateWithoutData::Running
                    }
                }
                Err(err) => StateWithoutData::Error(err),
            }
        }

        let result = match &mut self.state {
            State::Running(progress) => update_task(&mut self.distances, progress, new_start),
            State::Success(progress) => update_task(&mut self.distances, progress, new_start),
            State::Impossible(progress) => update_task(&mut self.distances, progress, new_start),
            _ => return Err(NoTarget),
        };
        match result {
            StateWithoutData::Success => self.state.success(),
            StateWithoutData::Running => self.state.running(),
            StateWithoutData::Error(err) => self.state = State::Error(err),
        }
        Ok(())
    }

    /// for debugging purposes
    pub fn n_active(&self) -> Option<(usize, usize)> {
        match &self.state {
            State::Ready => None,
            State::Running(progress) => {
                Some((progress.active_current.len(), progress.active_next.len()))
            }
            State::Success(progress) => {
                Some((progress.active_current.len(), progress.active_next.len()))
            }
            State::Error(_) => None,
            State::Impossible(progress) => {
                Some((progress.active_current.len(), progress.active_next.len()))
            }
        }
    }

    pub async fn run(&mut self, can_go: impl Fn(Position) -> bool) {
        if let State::Running(progress) = &mut self.state {
            match progress.run(&mut self.distances, can_go).await {
                Ok(NavigationResult::Impossible) => self.state.impossible(),
                Ok(NavigationResult::Success) => self.state.success(),
                Err(err) => self.state.error(err),
            }
        }
    }

    pub fn next_step(&self, pos: Position) -> DirectionCombination {
        let mut ret = DirectionCombination::default();
        if let Some(Some(dist_at)) = self.distances.get(pos) {
            for (neighbor, dir) in pos.neighbors() {
                if let Some(Some(dist_neighbor)) = self.distances.get(neighbor) {
                    if dist_neighbor < dist_at {
                        ret.set(dir, true);
                    }
                }
            }
        }
        ret
    }

    pub fn get_dist_at(&self, pos: Position) -> Option<u16> {
        distances_get(&self.distances, pos)
    }
}
