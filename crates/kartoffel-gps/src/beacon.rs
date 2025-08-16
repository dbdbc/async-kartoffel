use core::{convert::identity, future::Future, mem};

use alloc::boxed::Box;
use async_algorithm::{Breakpoint, DistanceManhattan, DistanceMeasure, DistanceMin};
use async_kartoffel_generic::{Direction, Vec2};

use heapless::{BinaryHeap, Vec, binary_heap::Min};

use crate::{GlobalPos, const_graph::Graph, map::TrueMap};

use core::marker::PhantomData;

/// TODO move to utility module
/// guaranteed to heap allocate an array without creating it on the stack first
pub fn heap_alloc_array<T: Clone, const N: usize>(t: T) -> Box<[T; N]> {
    alloc::vec::Vec::<T>::from_iter((0..N).map(|_| t.clone()))
        .into_boxed_slice()
        .try_into()
        .map_err(|_| ())
        .unwrap()
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Hash)]
pub struct BeaconInfo {
    pub max_beacon_dist: u32,

    /// from an arbitrary position, how many beacons are in sight (maximum)?
    pub max_beacons_entry: u32,

    /// for an arbitrary position, from how many beacons can it maximally be reached?
    pub max_beacons_exit: u32,

    /// what is the largest number of steps between any beacons
    pub max_path_length: u32,

    /// number of beacons
    pub n_beacons: u32,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Clone, Copy)]
pub enum Buffer {
    Path,
    TrivialNav,
    Active,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Clone, Copy)]
pub enum NavigatorError {
    OutOfMemory(Buffer),
    NavigationImpossible,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum UpdateType {
    // nothing to do
    TrivialNav,

    // Recompute, to account for longer distance travelled a few extra path nodes are popped
    Recompute { traveled_since: u16 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ScheduledUpdate {
    n_beacons_reached: u16,
    complexity: UpdateType,
}

impl ScheduledUpdate {
    fn new(old_start: GlobalPos, new_start: GlobalPos, trivial_dest: GlobalPos) -> Self {
        if new_start == trivial_dest {
            Self {
                n_beacons_reached: 1,
                complexity: UpdateType::TrivialNav,
            }
        } else if new_start == old_start {
            Self {
                n_beacons_reached: 0,
                complexity: UpdateType::TrivialNav,
            }
        } else {
            let movement = new_start - old_start;
            let moved_dirs = movement.directions();
            let movement_steps = DistanceManhattan::measure(movement);

            if movement_steps == 1
                && (trivial_dest - old_start)
                    .directions()
                    .contains(moved_dirs.first().unwrap())
            {
                // unwrap: there is exactly one dir
                // preferred case: single step in good dir, is really easy because of trivial
                // navigation rules
                Self {
                    n_beacons_reached: 0,
                    complexity: UpdateType::TrivialNav,
                }
            } else {
                // new calculation needed :(
                Self {
                    n_beacons_reached: 0,
                    complexity: UpdateType::Recompute {
                        traveled_since: movement_steps,
                    },
                }
            }
        }
    }

    fn update(
        &mut self,
        old_start: GlobalPos,
        new_start: GlobalPos,
        get_trivial_destination: impl FnOnce(u16) -> GlobalPos,
    ) {
        let trivial_dest = get_trivial_destination(self.n_beacons_reached);

        if new_start == trivial_dest {
            self.n_beacons_reached += 1;
            self.complexity = UpdateType::TrivialNav;
        } else if new_start == old_start {
            // no change
        } else {
            match self.complexity {
                UpdateType::TrivialNav => {
                    let movement = new_start - old_start;
                    let moved_dirs = movement.directions();
                    let movement_steps = DistanceManhattan::measure(movement);

                    if movement_steps == 1
                        && (trivial_dest - old_start)
                            .directions()
                            .contains(moved_dirs.first().unwrap())
                    {
                        // unwrap: there is exactly one dir
                        // preferred case: single step in good dir, is really easy because of trivial navigation rules
                        // no change
                    } else {
                        // new calculation needed
                        self.complexity = UpdateType::Recompute {
                            traveled_since: movement_steps,
                        }
                    }
                }
                UpdateType::Recompute { traveled_since } => {
                    self.complexity = UpdateType::Recompute {
                        traveled_since: traveled_since
                            + DistanceManhattan::measure(new_start - old_start),
                    };
                }
            }
        }
    }
}

pub struct NavigatorResourcesImpl<
    const MAX_PATH_LEN: usize,
    const MAX_ENTRY_EXIT: usize,
    const TRIV_BUFFER: usize,
    const NODE_BUFFER: usize,
    const ACTIVE_BUFFER: usize,
    T: TrueMap,
    G: Graph,
> {
    // constant
    context: NavigatorContext<T, G>,

    // buffers for computation
    buffers: NavigatorBuffers<MAX_ENTRY_EXIT, NODE_BUFFER, ACTIVE_BUFFER>,

    // buffer for resulting path in reverse order
    // path contains only beacons, not start and destination nodes
    path: Box<Vec<u16, MAX_PATH_LEN>>,

    _phantom: PhantomData<([(); MAX_PATH_LEN], [(); TRIV_BUFFER])>,
}

pub trait NavigatorResources {
    fn compute_new(
        &mut self,
        start: GlobalPos,
        destination: GlobalPos,
    ) -> impl Future<Output = Result<(), NavigatorError>> + Send;
    // Result<(), NavigatorError>;
    fn compute_update(
        &mut self,
        start: GlobalPos,
        destination: GlobalPos,
        update: ScheduledUpdate,
    ) -> impl Future<Output = Result<(), NavigatorError>> + Send;
    // Result<(), NavigatorError>;

    /// get the next beacon to navigate to, skipping n
    fn path_beacon(&self, n_skip: u16) -> Option<GlobalPos>;
    fn path_beacon_indices(&self) -> &[u16];
}

impl<
    const MAX_PATH_LEN: usize,
    const MAX_ENTRY_EXIT: usize,
    const TRIV_BUFFER: usize,
    const NODE_BUFFER: usize,
    const ACTIVE_BUFFER: usize,
    T: TrueMap + 'static,
    G: Graph + 'static,
> NavigatorResources
    for NavigatorResourcesImpl<
        MAX_PATH_LEN,
        MAX_ENTRY_EXIT,
        TRIV_BUFFER,
        NODE_BUFFER,
        ACTIVE_BUFFER,
        T,
        G,
    >
{
    async fn compute_new(
        &mut self,
        start: GlobalPos,
        destination: GlobalPos,
    ) -> Result<(), NavigatorError> {
        *self.path = Vec::new();
        compute::<MAX_PATH_LEN, MAX_ENTRY_EXIT, TRIV_BUFFER, NODE_BUFFER, ACTIVE_BUFFER, _, _>(
            start,
            destination,
            &mut self.buffers,
            self.context.clone(),
            &mut *self.path,
        )
        .await
    }

    async fn compute_update(
        &mut self,
        start: GlobalPos,
        destination: GlobalPos,
        update: ScheduledUpdate,
    ) -> Result<(), NavigatorError> {
        match update.complexity {
            UpdateType::TrivialNav => {
                self.path.truncate(
                    self.path
                        .len()
                        .saturating_sub(usize::from(update.n_beacons_reached)),
                );
                Ok(())
            }
            UpdateType::Recompute { traveled_since } => {
                // a heuristic for how many additional nodes should be popped for every tile
                // traveled since movement was no longer along trivial navigation route
                let n_pop_heuristic = traveled_since.div_ceil(4);

                self.path.truncate(
                    self.path
                        .len()
                        .saturating_sub(usize::from(update.n_beacons_reached))
                        .saturating_sub(usize::from(n_pop_heuristic)),
                );

                compute::<MAX_PATH_LEN, MAX_ENTRY_EXIT, TRIV_BUFFER, NODE_BUFFER, ACTIVE_BUFFER, _, _>(
                    start,
                    self.path
                        .last()
                        .map(|&index| self.context.beacons[usize::from(index)])
                        .unwrap_or(destination),
                    &mut self.buffers,
                    self.context.clone(),
                    &mut *self.path,
                )
                .await
            }
        }
    }

    fn path_beacon(&self, n_skip: u16) -> Option<GlobalPos> {
        // unwrap: path indices are expected to be valid beacons
        self.path
            .iter()
            .rev()
            .fuse()
            .nth(usize::from(n_skip))
            .map(|&index| *self.context.beacons.get(usize::from(index)).unwrap())
    }

    fn path_beacon_indices(&self) -> &[u16] {
        self.path.as_slice()
    }
}

impl<
    const MAX_PATH_LEN: usize,
    const MAX_ENTRY_EXIT: usize,
    const TRIV_BUFFER: usize,
    const NODE_BUFFER: usize,
    const ACTIVE_BUFFER: usize,
    T: TrueMap + 'static,
    G: Graph + 'static,
>
    NavigatorResourcesImpl<
        MAX_PATH_LEN,
        MAX_ENTRY_EXIT,
        TRIV_BUFFER,
        NODE_BUFFER,
        ACTIVE_BUFFER,
        T,
        G,
    >
{
    /// note: heap allocates memory for the computation buffers and path
    pub fn new(
        map: &'static T,
        graph: &'static G,
        beacons: &'static [GlobalPos],
        max_beacon_dist: u16,
    ) -> Self {
        Self {
            context: NavigatorContext {
                map,
                graph,
                beacons,
                max_beacon_dist,
            },
            buffers: NavigatorBuffers::new(),
            path: Default::default(),
            _phantom: PhantomData,
        }
    }
}

mod private {
    use crate::beacon::states;

    pub trait Sealed {}

    impl Sealed for states::New {}
    impl Sealed for states::OnlyDestination {}
    impl Sealed for states::OnlyStart {}
    impl Sealed for states::Initialized {}
    impl Sealed for states::Failed {}
    impl Sealed for states::Ready {}
    impl Sealed for states::UpdateScheduled {}
    impl Sealed for states::UpdateFailed {}
}

// finite state machine:
//
// New             -> set_start:       OnlyStart
//                 -> set_destination: OnlyDestination
//
// OnlyStart       -> set_destination: Initialized
//                 -> set_start:       OnlyStart
//                 -> reset:           New
//
// OnlyDestination -> set_start:       Initialized
//                 -> reset:           New
//                 -> set_destination: OnlyDestination
//
// Initialized     -> compute:         Ready | Failed | Completed
//                 -> set_start:       Initialized
//                 -> set_destination: Initialized
//                 -> reset:           New
//
// Failed          -> set_destination: Initialized
//                 -> set_start:       Initialized
//                 -> reset:           New
//
// Ready           -> set_destination: Initialized
//                 -> set_start:       UpdateScheduled
//                 -> reset:           New
//
// UpdateScheduled -> set_destination: Initialized
//                 -> set_start:       UpdateScheduled
//                 -> reset:           New
//                 -> compute:         Ready | UpdateFailed | Completed
//
// UpdateFailed    -> set_destination: Initialized
//                 -> set_start:       UpdateScheduled
//                 -> reset:           New
//
// Completed       -> set_destination: Initialized
//                 -> set_start:       Initialized
//                 -> reset:           New
//
// considerable computation effort is restricted to compute functions
//
pub mod states {
    use crate::{
        GlobalPos,
        beacon::{
            Navigator, NavigatorEnum, NavigatorError, NavigatorState, NavigatorStateFailed,
            NavigatorStateHasDestination, NavigatorStateHasDestinationNoPath,
            NavigatorStateHasPath, NavigatorStateHasStart, NavigatorStateResettable,
            ScheduledUpdate,
        },
    };

    // path is stored in shared state because of const generics ergonomics and potential buffer size
    pub struct New {}
    pub struct OnlyDestination {
        pub destination: GlobalPos,
    }
    pub struct OnlyStart {
        pub start: GlobalPos,
    }
    pub struct Failed {
        pub start: GlobalPos,
        pub destination: GlobalPos,
        pub error: NavigatorError,
    }
    pub struct Initialized {
        pub start: GlobalPos,
        pub destination: GlobalPos,
    }
    pub struct Ready {
        pub start: GlobalPos,
        pub destination: GlobalPos,
    }
    pub struct UpdateScheduled {
        pub start: GlobalPos,
        pub destination: GlobalPos,
        pub updates: ScheduledUpdate,
    }
    pub struct UpdateFailed {
        pub start: GlobalPos,
        pub destination: GlobalPos,
        pub updates: ScheduledUpdate,
        pub error: NavigatorError,
    }

    impl NavigatorState for New {
        fn to_enum<R: super::NavigatorResources>(nav: Navigator<R, Self>) -> NavigatorEnum<R> {
            NavigatorEnum::New(nav)
        }
    }
    impl NavigatorState for OnlyDestination {
        fn to_enum<R: super::NavigatorResources>(nav: Navigator<R, Self>) -> NavigatorEnum<R> {
            NavigatorEnum::OnlyDestination(nav)
        }
    }
    impl NavigatorState for OnlyStart {
        fn to_enum<R: super::NavigatorResources>(nav: Navigator<R, Self>) -> NavigatorEnum<R> {
            NavigatorEnum::OnlyStart(nav)
        }
    }
    impl NavigatorState for Failed {
        fn to_enum<R: super::NavigatorResources>(nav: Navigator<R, Self>) -> NavigatorEnum<R> {
            NavigatorEnum::Failed(nav)
        }
    }
    impl NavigatorState for Initialized {
        fn to_enum<R: super::NavigatorResources>(nav: Navigator<R, Self>) -> NavigatorEnum<R> {
            NavigatorEnum::Initialized(nav)
        }
    }
    impl NavigatorState for Ready {
        fn to_enum<R: super::NavigatorResources>(nav: Navigator<R, Self>) -> NavigatorEnum<R> {
            NavigatorEnum::Ready(nav)
        }
    }
    impl NavigatorState for UpdateScheduled {
        fn to_enum<R: super::NavigatorResources>(nav: Navigator<R, Self>) -> NavigatorEnum<R> {
            NavigatorEnum::UpdateScheduled(nav)
        }
    }
    impl NavigatorState for UpdateFailed {
        fn to_enum<R: super::NavigatorResources>(nav: Navigator<R, Self>) -> NavigatorEnum<R> {
            NavigatorEnum::UpdateFailed(nav)
        }
    }

    impl NavigatorStateResettable for OnlyDestination {}
    impl NavigatorStateResettable for OnlyStart {}
    impl NavigatorStateResettable for Failed {}
    impl NavigatorStateResettable for Initialized {}
    impl NavigatorStateResettable for Ready {}
    impl NavigatorStateResettable for UpdateScheduled {}
    impl NavigatorStateResettable for UpdateFailed {}

    impl NavigatorStateHasStart for OnlyStart {
        fn get_start(&self) -> GlobalPos {
            self.start
        }
    }
    impl NavigatorStateHasStart for Failed {
        fn get_start(&self) -> GlobalPos {
            self.start
        }
    }
    impl NavigatorStateHasStart for Initialized {
        fn get_start(&self) -> GlobalPos {
            self.start
        }
    }
    impl NavigatorStateHasStart for Ready {
        fn get_start(&self) -> GlobalPos {
            self.start
        }
    }
    impl NavigatorStateHasStart for UpdateScheduled {
        fn get_start(&self) -> GlobalPos {
            self.start
        }
    }
    impl NavigatorStateHasStart for UpdateFailed {
        fn get_start(&self) -> GlobalPos {
            self.start
        }
    }

    impl NavigatorStateHasDestination for OnlyDestination {
        fn get_destination(&self) -> GlobalPos {
            self.destination
        }
    }
    impl NavigatorStateHasDestination for Failed {
        fn get_destination(&self) -> GlobalPos {
            self.destination
        }
    }
    impl NavigatorStateHasDestination for Initialized {
        fn get_destination(&self) -> GlobalPos {
            self.destination
        }
    }
    impl NavigatorStateHasDestination for Ready {
        fn get_destination(&self) -> GlobalPos {
            self.destination
        }
    }
    impl NavigatorStateHasDestination for UpdateScheduled {
        fn get_destination(&self) -> GlobalPos {
            self.destination
        }
    }
    impl NavigatorStateHasDestination for UpdateFailed {
        fn get_destination(&self) -> GlobalPos {
            self.destination
        }
    }

    impl NavigatorStateHasDestinationNoPath for OnlyDestination {}
    impl NavigatorStateHasDestinationNoPath for Failed {}
    impl NavigatorStateHasDestinationNoPath for Initialized {}

    impl NavigatorStateFailed for Failed {
        fn get_error(&self) -> NavigatorError {
            self.error
        }
    }
    impl NavigatorStateFailed for UpdateFailed {
        fn get_error(&self) -> NavigatorError {
            self.error
        }
    }

    impl NavigatorStateHasPath for Ready {}
    impl NavigatorStateHasPath for UpdateScheduled {}
    impl NavigatorStateHasPath for UpdateFailed {}
}

// navigator state traits
pub trait NavigatorState: private::Sealed
where
    Self: Sized,
{
    fn to_enum<R: NavigatorResources>(nav: Navigator<R, Self>) -> NavigatorEnum<R>;
}
pub trait NavigatorStateResettable: NavigatorState {}
pub trait NavigatorStateHasStart: NavigatorState {
    fn get_start(&self) -> GlobalPos;
}
pub trait NavigatorStateHasDestination: NavigatorState {
    fn get_destination(&self) -> GlobalPos;
}
pub trait NavigatorStateHasDestinationNoPath: NavigatorStateHasDestination {}
pub trait NavigatorStateFailed: NavigatorState {
    fn get_error(&self) -> NavigatorError;
}
pub trait NavigatorStateHasPath: NavigatorState {
    fn get_beacons<R: NavigatorResources>(resources: &R) -> &[u16] {
        resources.path_beacon_indices()
    }
}

pub struct Navigator<R: NavigatorResources, S: NavigatorState> {
    resources: R,
    state: S,
}

// has anything
impl<R: NavigatorResources, S: NavigatorStateResettable> Navigator<R, S> {
    pub fn reset(self) -> Navigator<R, states::New> {
        Navigator {
            resources: self.resources,
            state: states::New {},
        }
    }
}

// has start
impl<R: NavigatorResources, S: NavigatorStateHasStart> Navigator<R, S> {
    pub fn set_destination(self, destination: GlobalPos) -> Navigator<R, states::Initialized> {
        Navigator {
            resources: self.resources,
            state: states::Initialized {
                start: self.state.get_start(),
                destination,
            },
        }
    }
    pub fn get_start(&self) -> GlobalPos {
        self.state.get_start()
    }
}

// has destination
impl<R: NavigatorResources, S: NavigatorStateHasDestination> Navigator<R, S> {
    pub fn get_destination(&self) -> GlobalPos {
        self.state.get_destination()
    }
}

// has destination and simple set_start implementation
impl<R: NavigatorResources, S: NavigatorStateHasDestinationNoPath> Navigator<R, S> {
    pub fn set_start(self, start: GlobalPos) -> Navigator<R, states::Initialized> {
        Navigator {
            resources: self.resources,
            state: states::Initialized {
                start,
                destination: self.state.get_destination(),
            },
        }
    }
}

// has failed
impl<R: NavigatorResources, S: NavigatorStateFailed> Navigator<R, S> {
    pub fn get_error(&self) -> NavigatorError {
        self.state.get_error()
    }
}

// has beacons
impl<R: NavigatorResources, S: NavigatorStateHasPath> Navigator<R, S> {
    pub fn get_beacons(&self) -> &[u16] {
        S::get_beacons(&self.resources)
    }
}

// exclusive to New
impl<R: NavigatorResources> Navigator<R, states::New> {
    pub fn new(resources: R) -> Self {
        Self {
            resources,
            state: states::New {},
        }
    }
    pub fn set_destination(self, destination: GlobalPos) -> Navigator<R, states::OnlyDestination> {
        Navigator {
            resources: self.resources,
            state: states::OnlyDestination { destination },
        }
    }
    pub fn set_start(self, start: GlobalPos) -> Navigator<R, states::OnlyStart> {
        Navigator {
            resources: self.resources,
            state: states::OnlyStart { start },
        }
    }
}

// exclusive to OnlyDestination
impl<R: NavigatorResources> Navigator<R, states::OnlyDestination> {
    pub fn set_destination(self, destination: GlobalPos) -> Navigator<R, states::OnlyDestination> {
        Navigator {
            resources: self.resources,
            state: states::OnlyDestination { destination },
        }
    }
}

// exclusive to OnlyStart
impl<R: NavigatorResources> Navigator<R, states::OnlyStart> {
    pub fn set_start(self, start: GlobalPos) -> Navigator<R, states::OnlyStart> {
        Navigator {
            resources: self.resources,
            state: states::OnlyStart { start },
        }
    }
}

// exclusive to Initialized
impl<R: NavigatorResources> Navigator<R, states::Initialized> {
    pub async fn compute(
        mut self,
    ) -> Result<Navigator<R, states::Ready>, Navigator<R, states::Failed>> {
        let (start, destination) = (self.state.start, self.state.destination);
        match self.resources.compute_new(start, destination).await {
            Ok(()) => Ok(Navigator {
                resources: self.resources,
                state: states::Ready { start, destination },
            }),
            Err(error) => Err(Navigator {
                resources: self.resources,
                state: states::Failed {
                    start,
                    destination,
                    error,
                },
            }),
        }
    }
}

// exclusive to Ready
impl<R: NavigatorResources> Navigator<R, states::Ready> {
    pub fn set_start(self, start: GlobalPos) -> Navigator<R, states::UpdateScheduled> {
        Navigator {
            state: states::UpdateScheduled {
                start,
                destination: self.state.destination,
                updates: ScheduledUpdate::new(self.state.start, start, self.next_trivial_target()),
            },
            resources: self.resources,
        }
    }

    pub fn next_trivial_target(&self) -> GlobalPos {
        self.resources
            .path_beacon(0)
            .unwrap_or(self.state.destination)
    }

    pub fn is_completed(&self) -> bool {
        self.state.start == self.state.destination
    }
}

// exclusive to UpdateScheduled
impl<R: NavigatorResources> Navigator<R, states::UpdateScheduled> {
    pub fn set_start(self, start: GlobalPos) -> Navigator<R, states::UpdateScheduled> {
        Navigator {
            state: states::UpdateScheduled {
                start,
                destination: self.state.destination,
                updates: {
                    let mut updates = self.state.updates;
                    updates.update(self.state.start, start, |n_skip| {
                        self.resources
                            .path_beacon(n_skip)
                            .unwrap_or(self.state.destination)
                    });
                    updates
                },
            },
            resources: self.resources,
        }
    }
    pub async fn compute(
        mut self,
    ) -> Result<Navigator<R, states::Ready>, Navigator<R, states::UpdateFailed>> {
        let (start, destination, updates) =
            (self.state.start, self.state.destination, self.state.updates);
        match self
            .resources
            .compute_update(start, destination, updates)
            .await
        {
            Ok(()) => Ok(Navigator {
                resources: self.resources,
                state: states::Ready { start, destination },
            }),
            Err(error) => Err(Navigator {
                resources: self.resources,
                state: states::UpdateFailed {
                    start,
                    destination,
                    updates,
                    error,
                },
            }),
        }
    }
}

// exclusive to UpdateFailed
impl<R: NavigatorResources> Navigator<R, states::UpdateFailed> {
    pub fn set_start(self, start: GlobalPos) -> Navigator<R, states::UpdateScheduled> {
        Navigator {
            state: states::UpdateScheduled {
                start,
                destination: self.state.destination,
                updates: {
                    let mut updates = self.state.updates;
                    updates.update(self.state.start, start, |n_skip| {
                        self.resources
                            .path_beacon(n_skip)
                            .unwrap_or(self.state.destination)
                    });
                    updates
                },
            },
            resources: self.resources,
        }
    }
}

pub enum NavigatorEnum<R: NavigatorResources> {
    New(Navigator<R, states::New>),
    OnlyStart(Navigator<R, states::OnlyStart>),
    OnlyDestination(Navigator<R, states::OnlyDestination>),
    Initialized(Navigator<R, states::Initialized>),
    Ready(Navigator<R, states::Ready>),
    Failed(Navigator<R, states::Failed>),
    UpdateFailed(Navigator<R, states::UpdateFailed>),
    UpdateScheduled(Navigator<R, states::UpdateScheduled>),
    Invalid,
}

impl<R: NavigatorResources> NavigatorEnum<R> {
    pub fn set_destination(&mut self, destination: GlobalPos) {
        *self = match mem::replace(self, Self::Invalid) {
            Self::New(nav) => nav.set_destination(destination).into(),
            Self::OnlyStart(nav) => nav.set_destination(destination).into(),
            Self::OnlyDestination(nav) => nav.set_destination(destination).into(),
            Self::Initialized(nav) => nav.set_destination(destination).into(),
            Self::Ready(nav) => nav.set_destination(destination).into(),
            Self::Failed(nav) => nav.set_destination(destination).into(),
            Self::UpdateFailed(nav) => nav.set_destination(destination).into(),
            Self::UpdateScheduled(nav) => nav.set_destination(destination).into(),
            Self::Invalid => unreachable!(),
        };
    }

    pub fn set_start(&mut self, start: GlobalPos) {
        *self = match mem::replace(self, Self::Invalid) {
            Self::New(nav) => nav.set_start(start).into(),
            Self::OnlyStart(nav) => nav.set_start(start).into(),
            Self::OnlyDestination(nav) => nav.set_start(start).into(),
            Self::Initialized(nav) => nav.set_start(start).into(),
            Self::Ready(nav) => nav.set_start(start).into(),
            Self::Failed(nav) => nav.set_start(start).into(),
            Self::UpdateFailed(nav) => nav.set_start(start).into(),
            Self::UpdateScheduled(nav) => nav.set_start(start).into(),
            Self::Invalid => unreachable!(),
        };
    }

    pub fn reset(&mut self) {
        *self = match mem::replace(self, Self::Invalid) {
            Self::New(nav) => nav.into(),
            Self::OnlyStart(nav) => nav.reset().into(),
            Self::OnlyDestination(nav) => nav.reset().into(),
            Self::Initialized(nav) => nav.reset().into(),
            Self::Ready(nav) => nav.reset().into(),
            Self::Failed(nav) => nav.reset().into(),
            Self::UpdateFailed(nav) => nav.reset().into(),
            Self::UpdateScheduled(nav) => nav.reset().into(),
            Self::Invalid => unreachable!(),
        };
    }

    /// result does not specify whether the computation succeded, but only if the initial state was
    /// one where a computation was possible
    pub async fn try_compute(&mut self) -> bool {
        let success;
        (success, *self) = match mem::replace(self, Self::Invalid) {
            Self::Initialized(nav) => (true, nav.compute().await.into()),
            Self::UpdateScheduled(nav) => (true, nav.compute().await.into()),
            owned => (false, owned),
        };
        success
    }

    pub fn try_get_start(&self) -> Option<GlobalPos> {
        match self {
            Self::New(_nav) => None,
            Self::OnlyStart(nav) => Some(nav.get_start()),
            Self::OnlyDestination(_nav) => None,
            Self::Initialized(nav) => Some(nav.get_start()),
            Self::Ready(nav) => Some(nav.get_start()),
            Self::Failed(nav) => Some(nav.get_start()),
            Self::UpdateFailed(nav) => Some(nav.get_start()),
            Self::UpdateScheduled(nav) => Some(nav.get_start()),
            Self::Invalid => unreachable!(),
        }
    }

    pub fn try_get_destination(&self) -> Option<GlobalPos> {
        match self {
            Self::New(_nav) => None,
            Self::OnlyStart(_nav) => None,
            Self::OnlyDestination(nav) => Some(nav.get_destination()),
            Self::Initialized(nav) => Some(nav.get_destination()),
            Self::Ready(nav) => Some(nav.get_destination()),
            Self::Failed(nav) => Some(nav.get_destination()),
            Self::UpdateFailed(nav) => Some(nav.get_destination()),
            Self::UpdateScheduled(nav) => Some(nav.get_destination()),
            Self::Invalid => unreachable!(),
        }
    }

    pub fn try_get_error(&self) -> Option<NavigatorError> {
        match self {
            Self::Failed(nav) => Some(nav.get_error()),
            Self::UpdateFailed(nav) => Some(nav.get_error()),
            _ => None,
        }
    }

    pub fn try_next_trivial_target(&self) -> Option<GlobalPos> {
        if let Self::Ready(nav) = self {
            Some(nav.next_trivial_target())
        } else {
            None
        }
    }

    pub fn try_get_beacons(&self) -> Option<&[u16]> {
        match self {
            Self::Ready(nav) => Some(nav.get_beacons()),
            Self::UpdateFailed(nav) => Some(nav.get_beacons()),
            Self::UpdateScheduled(nav) => Some(nav.get_beacons()),
            _ => None,
        }
    }
}

impl<R: NavigatorResources, S: NavigatorState> From<Navigator<R, S>> for NavigatorEnum<R> {
    fn from(value: Navigator<R, S>) -> Self {
        S::to_enum(value)
    }
}

impl<R: NavigatorResources, S1: NavigatorState, S2: NavigatorState>
    From<Result<Navigator<R, S1>, Navigator<R, S2>>> for NavigatorEnum<R>
{
    fn from(value: Result<Navigator<R, S1>, Navigator<R, S2>>) -> Self {
        match value {
            Ok(nav) => nav.into(),
            Err(nav) => nav.into(),
        }
    }
}

// Ord derived implementation ensures desired `Min` behaviour for the priority queue
#[derive(PartialEq, Eq, Debug)]
struct NavActiveEntry {
    estimated_cost: u16,
    past_cost: u16,
    node: Node,
}

impl PartialOrd for NavActiveEntry {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NavActiveEntry {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        match self.estimated_cost.cmp(&other.estimated_cost) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.past_cost.cmp(&other.past_cost) {
            core::cmp::Ordering::Equal => {}
            ord => return ord.reverse(), // note the reverse, because we want to prioritize paths
                                         // that already crossed a larger distance
        }
        self.node.cmp(&other.node)
    }
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash)]
enum Node {
    Beacon(u16),
    Start,
    Destination,
}

// Buffers that can be used for computations
struct NavigatorBuffers<
    const MAX_ENTRY_EXIT: usize,
    const NODE_BUFFER: usize,
    const ACTIVE_BUFFER: usize,
> {
    entry_nodes: Box<Vec<u16, MAX_ENTRY_EXIT>>,
    exit_nodes: Box<Vec<u16, MAX_ENTRY_EXIT>>,
    active: Box<BinaryHeap<NavActiveEntry, Min, ACTIVE_BUFFER>>,
    node_info: Box<[Option<(u16, Node)>; NODE_BUFFER]>,
}

impl<const MAX_ENTRY_EXIT: usize, const NODE_BUFFER: usize, const ACTIVE_BUFFER: usize>
    NavigatorBuffers<MAX_ENTRY_EXIT, NODE_BUFFER, ACTIVE_BUFFER>
{
    fn reset(&mut self) {
        self.entry_nodes.clear();
        self.exit_nodes.clear();
        self.active.clear();
        self.node_info.fill_with(|| None);
    }

    /// note: allocates heap memory
    /// inlining is prevented to be able to check stack usage
    #[inline(never)]
    pub fn new() -> Self {
        Self {
            entry_nodes: Default::default(),
            exit_nodes: Default::default(),
            active: Box::new(BinaryHeap::new()),
            node_info: heap_alloc_array(None),
        }
    }
}

impl<const MAX_ENTRY_EXIT: usize, const NODE_BUFFER: usize, const ACTIVE_BUFFER: usize> Default
    for NavigatorBuffers<MAX_ENTRY_EXIT, NODE_BUFFER, ACTIVE_BUFFER>
{
    fn default() -> Self {
        Self {
            entry_nodes: Default::default(),
            exit_nodes: Default::default(),
            active: Default::default(),
            node_info: Box::new([None; NODE_BUFFER]),
        }
    }
}

/// const navigator state
pub struct NavigatorContext<T: TrueMap, G: Graph> {
    map: &'static T,
    graph: &'static G,
    beacons: &'static [GlobalPos],
    max_beacon_dist: u16,
}

impl<T: TrueMap, G: Graph> Clone for NavigatorContext<T, G> {
    fn clone(&self) -> Self {
        Self {
            map: self.map,
            graph: self.graph,
            beacons: self.beacons,
            max_beacon_dist: self.max_beacon_dist,
        }
    }
}

/// assumes that start is walkable
/// assumes distances to be small enough to fit in i16
/// BUFFER_SIZE needs to be at least (dist_manhattan + 2).div_floor(2)
/// this functions makes no assumptions about BUFFER_SIZE, but returns an Err
fn is_navigation_trivial<const BUFFER_SIZE: usize>(
    map: &impl TrueMap,
    start: GlobalPos,
    destination: GlobalPos,
) -> Result<bool, NavigatorError> {
    let vector = destination - start;
    let dirs = {
        let mut dirs = Vec::<Direction, 2>::new();
        // unwrap: there can only be two dirs to add
        match vector.east() {
            ..=-1 => dirs.push(Direction::West).unwrap(),
            0 => (),
            1.. => dirs.push(Direction::East).unwrap(),
        }
        match vector.south() {
            ..=-1 => dirs.push(Direction::North).unwrap(),
            0 => (),
            1.. => dirs.push(Direction::South).unwrap(),
        }
        dirs
    };

    let dist_man = DistanceManhattan::measure(vector);
    let dist_min = DistanceMin::measure(vector);

    if dirs.is_empty() {
        // start == destination
        Ok(true)
    } else if dirs.len() == 1 {
        // straight line, example
        // S X X X D
        // dist_man: 4
        // unwrap: all distances are expected to fit in i16
        for i in 1..=i16::try_from(dist_man).unwrap() {
            if !map.get(start + Vec2::new_in_direction(dirs[0], i)) {
                return Ok(false);
            }
        }
        Ok(true)
    } else {
        // unwrap: all distances are expected to fit in i16, vector in dir is nonnegative
        let max_dir_0 = u16::try_from(vector.in_direction(dirs[0])).unwrap();
        let max_dir_1 = dist_man - max_dir_0;

        // out of bounds
        if usize::from(dist_min + 1) > BUFFER_SIZE {
            return Err(NavigatorError::OutOfMemory(Buffer::TrivialNav));
        };

        let mut actives_next = [false; BUFFER_SIZE];
        actives_next[0] = true;

        //   1 2 3 3 3
        //  / / / / / 2
        // S X X X X /1
        // X X X X X /
        // X X X X D
        // dist_min: 2
        // dist_man: 6

        // returns index_th position in rect with given distance to start
        let pos_from_manhattan_and_index = |i_manhattan: u16, i_index: u16| {
            let dist_dir_0 = i_manhattan.min(max_dir_0);
            let dist_dir_1 = i_manhattan - dist_dir_0;
            // unwrap: distances fit in i16
            start
                + Vec2::new_in_direction(dirs[0], i16::try_from(dist_dir_0 - i_index).unwrap())
                + Vec2::new_in_direction(dirs[1], i16::try_from(dist_dir_1 + i_index).unwrap())
        };

        let neighbor_indices = |i_manhattan: u16, i_index: u16| {
            let i_0 = i_manhattan.min(max_dir_0) - i_index;
            let i_1 = i_manhattan - i_0;

            let index_offset = if i_manhattan >= max_dir_0 { 1 } else { 0 };

            // unwrap: there can only be two neighbors
            let mut next = Vec::<u16, 2>::new();
            if i_0 < max_dir_0 {
                next.push(i_index - index_offset).unwrap();
            }
            if i_1 < max_dir_1 {
                next.push(i_index + 1 - index_offset).unwrap();
            }
            next
        };

        let mut actives: [bool; BUFFER_SIZE];
        for i_manhattan in 0..dist_man {
            actives = actives_next;
            actives_next = [false; BUFFER_SIZE];

            // number of locations on this diagonal that might be active
            let n_to_check = dist_min.min(i_manhattan).min(dist_man - i_manhattan) + 1;

            for index_to_check in 0..n_to_check {
                if actives[usize::from(index_to_check)] {
                    let next_indices: Vec<u16, 2> = neighbor_indices(i_manhattan, index_to_check)
                        .into_iter()
                        .filter(|&i| {
                            let pos = pos_from_manhattan_and_index(i_manhattan + 1, i);
                            map.get(pos)
                        })
                        .collect();
                    if next_indices.is_empty() {
                        // we reached a dead end
                        return Ok(false);
                    }
                    for i in next_indices {
                        actives_next[usize::from(i)] = true;
                    }
                }
            }
        }
        Ok(true)
    }
}

/// calculates the fastes path from start to destination and appends it to the path (path is in
/// reverse order, so the newly computed parts are actually resolved first)
///
/// buffers are reset at the start of this function, so their content does not matter
///
/// TODO check for no exit nodes for faster error if destination is not walkable
async fn compute<
    const MAX_PATH_LEN: usize,
    const MAX_ENTRY_EXIT: usize,
    const TRIV_BUFFER: usize,
    const NODE_BUFFER: usize,
    const ACTIVE_BUFFER: usize,
    T: TrueMap,
    G: Graph,
>(
    start: GlobalPos,
    destination: GlobalPos,
    buffers: &mut NavigatorBuffers<MAX_ENTRY_EXIT, NODE_BUFFER, ACTIVE_BUFFER>,
    context: NavigatorContext<T, G>,
    path: &mut Vec<u16, MAX_PATH_LEN>, // path in reverse order, calculation is appended
) -> Result<(), NavigatorError> {
    // clear intermediate state
    buffers.reset();
    let mut node_info_destination: Option<(u16, Node)> = None;

    // entry
    *buffers.entry_nodes = context
        .beacons
        .iter()
        .enumerate()
        .filter(|&(_, &pos)| DistanceManhattan::measure(pos - start) <= context.max_beacon_dist)
        .filter(|&(_, &pos)| {
            // possible OutOfMemory error ignored here, but thats ok because it can only appear
            // if TRIV_BUFFER is misconfigured
            is_navigation_trivial::<TRIV_BUFFER>(context.map, start, pos).is_ok_and(identity)
        })
        .map(|(index, _)| u16::try_from(index).unwrap())
        .collect();

    Breakpoint::new().await;

    // exit
    *buffers.exit_nodes = context
        .beacons
        .iter()
        .enumerate()
        .filter(|&(_, &pos)| {
            DistanceManhattan::measure(destination - pos) <= context.max_beacon_dist
        })
        .filter(|&(_, &pos)| {
            is_navigation_trivial::<TRIV_BUFFER>(context.map, pos, destination).unwrap()
        }) // TODO unwrap
        .map(|(index, _)| u16::try_from(index).unwrap())
        .collect();

    Breakpoint::new().await;

    if start == destination
        || (DistanceManhattan::measure(destination - start) <= context.max_beacon_dist
            && is_navigation_trivial::<TRIV_BUFFER>(context.map, start, destination)
                .map_err(|_| NavigatorError::OutOfMemory(Buffer::TrivialNav))?)
    {
        // nothing to add to path, navigation from start to destination is trivial
        Ok(())
    } else {
        // graph initialization
        for &node_index in &*buffers.entry_nodes {
            let pos = context.beacons[usize::from(node_index)];
            let past_cost = DistanceManhattan::measure(pos - start);

            buffers
                .active
                .push(NavActiveEntry {
                    estimated_cost: past_cost + DistanceManhattan::measure(destination - pos),
                    past_cost,
                    node: Node::Beacon(node_index),
                })
                .map_err(|_| NavigatorError::OutOfMemory(Buffer::Active))?;
            buffers.node_info[usize::from(node_index)] = Some((past_cost, Node::Start));
        }

        Breakpoint::new().await;

        // graph traversal
        let mut counter: u8 = 0;
        'main_loop: while let Some(NavActiveEntry {
            estimated_cost: _,
            past_cost,
            node,
        }) = buffers.active.pop()
        {
            match node {
                Node::Start => core::unreachable!("start is never added to the active nodes"),
                Node::Destination => {
                    break 'main_loop;
                }
                Node::Beacon(node_index) => {
                    let pos = context.beacons[usize::from(node_index)];

                    // this check ensures that nodes that were added multiple time are only
                    // processed once and might not be necessary
                    if buffers.node_info[usize::from(node_index)]
                        .is_none_or(|(past_cost_ni, _)| past_cost_ni == past_cost)
                    {
                        // neighbor is destination node
                        if buffers.exit_nodes.contains(&node_index) {
                            let total_cost =
                                past_cost + DistanceManhattan::measure(destination - pos);
                            if let Some((total_cost_old, parent)) = &mut node_info_destination {
                                if total_cost < *total_cost_old {
                                    *total_cost_old = total_cost;
                                    *parent = node;
                                }

                                buffers
                                    .active
                                    .push(NavActiveEntry {
                                        estimated_cost: total_cost,
                                        past_cost: total_cost,
                                        node: Node::Destination,
                                    })
                                    .unwrap();
                            } else {
                                node_info_destination = Some((total_cost, node));

                                buffers
                                    .active
                                    .push(NavActiveEntry {
                                        estimated_cost: total_cost,
                                        past_cost: total_cost,
                                        node: Node::Destination,
                                    })
                                    .unwrap();
                            }
                        }

                        // neighbors are beacon nodes
                        for &neighbor in context.graph.after(node_index) {
                            let pos_neighbor = context.beacons[usize::from(neighbor)];
                            let past_cost_neighbor =
                                past_cost + DistanceManhattan::measure(pos_neighbor - pos);
                            if let Some((past_cost_old, parent)) =
                                &mut (&mut buffers.node_info)[usize::from(neighbor)]
                            {
                                if past_cost_neighbor < *past_cost_old {
                                    *past_cost_old = past_cost_neighbor;
                                    *parent = node;

                                    buffers
                                        .active
                                        .push(NavActiveEntry {
                                            estimated_cost: past_cost_neighbor
                                                + DistanceManhattan::measure(
                                                    destination - pos_neighbor,
                                                ),
                                            past_cost: past_cost_neighbor,
                                            node: Node::Beacon(neighbor),
                                        })
                                        .unwrap();
                                }
                            } else {
                                buffers.node_info[usize::from(neighbor)] =
                                    Some((past_cost_neighbor, node));

                                buffers
                                    .active
                                    .push(NavActiveEntry {
                                        estimated_cost: past_cost_neighbor
                                            + DistanceManhattan::measure(
                                                destination - pos_neighbor,
                                            ),
                                        past_cost: past_cost_neighbor,
                                        node: Node::Beacon(neighbor),
                                    })
                                    .unwrap();
                            }
                        }
                    }
                }
            };

            counter = counter.wrapping_add(1);
            if counter.rem_euclid(16) == 0 {
                Breakpoint::new().await;
            }
        }

        Breakpoint::new().await;

        // path collection
        if let &Some((ref _cost, mut parent_node)) = &node_info_destination {
            loop {
                match parent_node {
                    Node::Start => break,
                    Node::Destination => core::unreachable!(),
                    Node::Beacon(beacon_index) => {
                        // prevent duplicates, that can happen e.g. through path updates
                        if path
                            .last()
                            .is_none_or(|&last_index| last_index != beacon_index)
                        {
                            path.push(beacon_index)
                                .map_err(|_| NavigatorError::OutOfMemory(Buffer::Path))?;
                        }
                        // unwrap: the node must have appeared while traversing the graph
                        parent_node = (&buffers.node_info)[usize::from(beacon_index)].unwrap().1;
                    }
                }
            }
            Ok(())
        } else {
            Err(NavigatorError::NavigationImpossible)
        }
    }
}

#[cfg(test)]
mod tests {

    use core::{
        fmt::{Display, Write},
        sync::atomic::{AtomicBool, Ordering},
    };

    use async_kartoffel::{Global, print, println};
    use rand::{
        SeedableRng,
        distr::{Distribution, Uniform},
        rngs::SmallRng,
        seq::IteratorRandom,
    };

    use crate::pos::pos_east_south;

    extern crate alloc;

    use super::*;
    use test_kartoffel::{
        TestError, assert, assert_eq, assert_err, assert_none, option_unwrap, result_unwrap,
    };

    struct TestMap<const WIDTH: usize, const HEIGHT: usize> {
        tiles: [[bool; WIDTH]; HEIGHT],
        dirty_outside: AtomicBool,
    }

    impl<const WIDTH: usize, const HEIGHT: usize> Display for TestMap<WIDTH, HEIGHT> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            for i_h in 0..HEIGHT {
                for i_w in 0..WIDTH {
                    f.write_char(if self.tiles[i_h][i_w] { '.' } else { '#' })?
                }
                f.write_char('\n')?;
            }
            core::fmt::Result::Ok(())
        }
    }

    impl<const WIDTH: usize, const HEIGHT: usize> TestMap<WIDTH, HEIGHT> {
        fn new_like(&self) -> Self {
            Self {
                tiles: [[false; WIDTH]; HEIGHT],
                dirty_outside: AtomicBool::new(false),
            }
        }

        fn new(tiles: [[bool; WIDTH]; HEIGHT]) -> Self {
            Self {
                tiles,
                dirty_outside: AtomicBool::new(false),
            }
        }

        fn corner_north_west(&self) -> GlobalPos {
            pos_east_south(0, 0)
        }

        fn corner_north_east(&self) -> GlobalPos {
            pos_east_south(WIDTH as i16 - 1, 0)
        }

        fn corner_south_west(&self) -> GlobalPos {
            pos_east_south(0, HEIGHT as i16 - 1)
        }

        fn corner_south_east(&self) -> GlobalPos {
            pos_east_south(WIDTH as i16 - 1, HEIGHT as i16 - 1)
        }

        fn set(&mut self, pos: GlobalPos, val: bool) -> Result<(), ()> {
            let vec = pos.subtract_anchor();
            let east = vec.east();
            let south = vec.south();
            if east < 0 || east >= self.width() as i16 || south < 0 || south >= self.height() as i16
            {
                Err(())
            } else {
                self.tiles[south as usize][east as usize] = val;
                Ok(())
            }
        }
    }

    impl<const WIDTH: usize, const HEIGHT: usize> TrueMap for TestMap<WIDTH, HEIGHT> {
        fn get(&self, pos: GlobalPos) -> bool {
            let vec = pos.subtract_anchor();
            let east = vec.east();
            let south = vec.south();
            if east < 0 || east >= self.width() as i16 || south < 0 || south >= self.height() as i16
            {
                self.dirty_outside.store(true, Ordering::Relaxed);
                false
            } else {
                self.tiles[south as usize][east as usize]
            }
        }

        fn vec_east(&self) -> Vec2<Global> {
            Vec2::new_east(WIDTH as i16)
        }

        fn vec_south(&self) -> Vec2<Global> {
            Vec2::new_south(HEIGHT as i16)
        }

        fn width(&self) -> u16 {
            WIDTH as u16
        }

        fn height(&self) -> u16 {
            HEIGHT as u16
        }
    }

    #[test_case]
    fn trivial_nav1() -> Result<(), TestError> {
        println!("map1");
        let map = TestMap::new([
            [true, true, true, true, true, true],
            [true, true, true, true, true, true],
            [true, true, true, true, true, true],
            [true, true, true, true, true, true],
            [true, true, true, true, true, true],
            [true, true, true, true, true, true],
        ]);
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_north_west(), map.corner_south_east()),
            Ok(true)
        );
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_south_east(), map.corner_north_west()),
            Ok(true)
        );
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_south_west(), map.corner_north_east()),
            Ok(true)
        );
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_north_east(), map.corner_south_west()),
            Ok(true)
        );
        assert_eq!(map.dirty_outside.load(Ordering::Relaxed), false);

        Ok(())
    }

    #[test_case]
    fn trivial_nav2() -> Result<(), TestError> {
        println!("map2");
        let map = TestMap::new([
            [true, true, true, true, true, true],
            [true, false, true, true, false, true],
            [true, true, false, false, true, true],
            [true, false, true, true, false, true],
            [true, true, true, true, true, true],
        ]);
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_north_west(), map.corner_south_east()),
            Ok(false)
        );
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_south_east(), map.corner_north_west()),
            Ok(false)
        );
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_south_west(), map.corner_north_east()),
            Ok(false)
        );
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_north_east(), map.corner_south_west()),
            Ok(false)
        );
        assert_eq!(map.dirty_outside.load(Ordering::Relaxed), false);

        Ok(())
    }

    #[test_case]
    fn trivial_nav3() -> Result<(), TestError> {
        println!("map3");
        let map = TestMap::new([
            [true, true, true],
            [true, true, true],
            [true, true, true],
            [true, false, true],
        ]);
        assert_eq!(map.get(pos_east_south(2, -3)), true);
        assert_eq!(map.get(pos_east_south(1, -3)), false);
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_north_west(), map.corner_south_east()),
            Ok(false)
        );
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_south_east(), map.corner_north_west()),
            Ok(true)
        );
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_south_west(), map.corner_north_east()),
            Ok(true)
        );
        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_north_east(), map.corner_south_west()),
            Ok(false)
        );
        assert_eq!(map.dirty_outside.load(Ordering::Relaxed), false);

        assert_eq!(
            is_navigation_trivial::<64>(&map, map.corner_north_east(), pos_east_south(3, -3)),
            Ok(false)
        );
        assert_eq!(map.dirty_outside.load(Ordering::Relaxed), true);

        Ok(())
    }

    /// Trivial navigable means: No matter where you go, if the general direction is correct, you will
    /// reach your destination.
    fn is_trivial_navigable_test<const WIDTH: usize, const HEIGHT: usize>(
        map: &TestMap<WIDTH, HEIGHT>,
        xx_start: GlobalPos,
        destination: GlobalPos,
    ) -> bool {
        let mut start = map.new_like();

        if !map.get(destination) {
            return false;
        }

        start.set(destination, true).unwrap();
        if destination == xx_start {
            return true;
        }

        // fill for single direction
        for dir in Direction::all() {
            let mut i = 1i16;
            loop {
                let pos = destination + Vec2::new_in_direction(dir, i);
                if map.get(pos) {
                    start.set(pos, true).unwrap();
                    if pos == xx_start {
                        return true;
                    }
                    i += 1;
                } else {
                    break;
                }
            }
        }

        for dir_ew in [Direction::East, Direction::West] {
            for dir_ns in [Direction::North, Direction::South] {
                // recursive condition:
                // - all walkable neighbors in the two directions must be trivially navigable from
                // - at least on neighbor in the two directions must be walkable
                // => all walkable neighbors in the two directions must have been checked before
                // => diagonal iteration with increasing manhattan dist

                let mut i_manhattan = 2i16;
                loop {
                    let mut any: bool = false;
                    any =
                        any || start.get(destination + Vec2::new_in_direction(dir_ew, i_manhattan));
                    any =
                        any || start.get(destination + Vec2::new_in_direction(dir_ns, i_manhattan));

                    for i_ew in 1..=i_manhattan - 1 {
                        let i_ns = i_manhattan - i_ew;
                        let pos = destination
                            + Vec2::new_in_direction(dir_ew, i_ew)
                            + Vec2::new_in_direction(dir_ns, i_ns);
                        let neighbor_ew = pos - Vec2::new_in_direction(dir_ew, 1);
                        let neighbor_ns = pos - Vec2::new_in_direction(dir_ns, 1);

                        let walk = map.get(pos);
                        let triv_ns = start.get(neighbor_ns);
                        let triv_ew = start.get(neighbor_ew);
                        let wall_ns = !map.get(neighbor_ns);
                        let wall_ew = !map.get(neighbor_ew);

                        if walk
                            && (triv_ns || wall_ns)
                            && (triv_ew || wall_ew)
                            && (triv_ew || triv_ns)
                        {
                            start.set(pos, true).unwrap();
                            if pos == xx_start {
                                return true;
                            }
                            any = true;
                        }
                    }
                    if !any {
                        break;
                    }
                    i_manhattan += 1;
                }
            }
        }
        return false;
    }

    #[test_case]
    fn trivial_nav_positive() -> Result<(), TestError> {
        let mut rng = {
            let seed = [0u8; 32];
            let rng = SmallRng::from_seed(seed);
            rng
        };

        // (max_dist + 2).div_ceil(2).next_power_of_two(),
        fn for_size<const WIDTH: usize, const HEIGHT: usize, const BUFFER_SIZE: usize>(
            rng: &mut SmallRng,
            odds: (u16, u16),
        ) -> Result<(), TestError> {
            let mut map = [[false; WIDTH]; HEIGHT];
            let dist = Uniform::try_from(0..odds.0 + odds.1).unwrap();

            for i_h in 0..HEIGHT {
                for i_w in 0..WIDTH {
                    map[i_h][i_w] = dist.sample(rng) < odds.0;
                }
            }

            let map = TestMap::new(map);

            println!("{}", map);

            for i in 0..4 {
                let (start, dest, dir0, dir1) = match i {
                    0 => (
                        map.corner_north_west(),
                        map.corner_south_east(),
                        Direction::South,
                        Direction::East,
                    ),
                    1 => (
                        map.corner_south_west(),
                        map.corner_north_east(),
                        Direction::North,
                        Direction::East,
                    ),
                    2 => (
                        map.corner_south_east(),
                        map.corner_north_west(),
                        Direction::North,
                        Direction::West,
                    ),
                    3 => (
                        map.corner_north_east(),
                        map.corner_south_west(),
                        Direction::South,
                        Direction::West,
                    ),
                    _ => unreachable!(),
                };

                let triv_nav = is_navigation_trivial::<BUFFER_SIZE>(&map, start, dest).unwrap();

                if triv_nav {
                    println!("t {:?} {:?}", dir0, dir1);
                    let mut pos = start;
                    for i in 0..WIDTH + HEIGHT - 2 {
                        let dirs = [dir0, dir1].into_iter().filter(|&dir| {
                            let new_pos = pos + Vec2::new_in_direction(dir, 1);
                            (dest - new_pos).in_direction(dir) >= 0 && map.get(new_pos)
                        });
                        let dir = dirs.choose(rng);
                        if let Some(dir) = dir {
                            pos = pos + Vec2::new_in_direction(dir, 1);
                        } else {
                            return Err(TestError);
                        }
                    }
                    assert_eq!(pos, dest);
                    assert_eq!(
                        is_trivial_navigable_test(&map, start, dest) || !map.get(start),
                        true
                    );
                } else {
                    println!("n {:?} {:?}", dir0, dir1);
                    assert_eq!(is_trivial_navigable_test(&map, start, dest), false);
                }
            }

            // assert_eq!(map.dirty_outside.get(), false);
            println!();

            Ok(())
        }

        for i in 0..100 {
            println!("\n{}", i);
            for_size::<1, 5, 3>(&mut rng, (4, 1))?;
            for_size::<2, 5, 3>(&mut rng, (4, 1))?;
            for_size::<3, 5, 4>(&mut rng, (4, 1))?;
            for_size::<4, 5, 4>(&mut rng, (4, 1))?;
            for_size::<5, 5, 5>(&mut rng, (4, 1))?;
            for_size::<5, 4, 4>(&mut rng, (4, 1))?;
            for_size::<5, 3, 4>(&mut rng, (4, 1))?;
            for_size::<5, 2, 3>(&mut rng, (4, 1))?;
            for_size::<5, 1, 3>(&mut rng, (4, 1))?;
        }

        Ok(())
    }
}
