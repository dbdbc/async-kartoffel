#![no_main]
#![no_std]
#![feature(custom_test_frameworks)]
#![test_runner(test_kartoffel::runner)]
#![feature(iter_next_chunk)]

use async_algorithm::{Breakpoint, DistanceManhattan, DistanceMeasure, StatsDog};
use async_kartoffel::{
    Arm, Bot, Instant, KartoffelClock, Motor, Radar, RadarScan, Timer, exit, println,
};
use async_kartoffel_generic::{
    D7, Direction, Duration, Local, Position, PositionAnchor, RadarScanTrait, RadarSize, Rotation,
    Tile, Vec2,
};
use embassy_executor::{Executor, task};
use embassy_futures::select::{Either, Either3, select, select3};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel, signal::Signal};
use example_kartoffels::{get_global_pos, navigator_resources};
use heapless::Vec;
use kartoffel_gps::{
    GlobalPos,
    beacon::{Navigator, NavigatorEnum, NavigatorError},
    gps::{MapSection, MapSectionTrait},
    pos::pos_east_south,
};
use static_cell::StaticCell;

extern crate alloc;

#[unsafe(no_mangle)]
fn main() {
    static EXECUTOR: StaticCell<Executor> = StaticCell::new();
    static CHANNEL_POSITION: StaticCell<Channel<NoopRawMutex, GlobalPos, 16>> = StaticCell::new();
    static SIGNAL_NAVIGATION: StaticCell<
        Signal<NoopRawMutex, Result<NavigationSection, NavigatorError>>,
    > = StaticCell::new();
    static SIGNAL_DESTINATION: StaticCell<Signal<NoopRawMutex, GlobalPos>> = StaticCell::new();
    static SIGNAL_RESET: StaticCell<Signal<NoopRawMutex, ()>> = StaticCell::new();
    static SIGNAL_COMPLETE: StaticCell<Signal<NoopRawMutex, ()>> = StaticCell::new();

    let channel_position = CHANNEL_POSITION.init(Channel::new());
    let signal_navigation = SIGNAL_NAVIGATION.init(Signal::new());
    let signal_destination = SIGNAL_DESTINATION.init(Signal::new());
    let signal_reset = SIGNAL_RESET.init(Signal::new());
    let signal_complete = SIGNAL_COMPLETE.init(Signal::new());

    let executor = EXECUTOR.init(Executor::new());

    println!("async_kartoffel");
    println!("gps navigation test");

    executor.run(|spawner| {
        spawner
            .spawn(foreground(
                Bot::take(),
                DataSync {
                    channel_position,
                    signal_navigation,
                    signal_destination,
                    signal_reset,
                },
                signal_complete,
            ))
            .unwrap();
        spawner
            .spawn(navigation(DataSync {
                channel_position,
                signal_navigation,
                signal_destination,
                signal_reset,
            }))
            .unwrap();
        spawner.spawn(watchdog(signal_complete)).unwrap();
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct NavigationSection {
    trivial_dest: GlobalPos,
    start: GlobalPos,
}

impl NavigationSection {
    /// update if new position is available, only handles trivial navigation (= right direction single step)
    /// updates
    fn update(self_: &mut Option<Self>, pos: GlobalPos) {
        if let Some(section) = self_ {
            let movement = pos - section.start;
            let dist = DistanceManhattan::measure(movement);
            if dist == 1
                && (section.trivial_dest - section.start)
                    .directions()
                    .contains(movement.directions().first().unwrap())
            {
                // single step in the right direction to keep trivial navigation invariant
                section.start = pos;
            }
        }
    }

    fn directions(&self) -> &'static [Direction] {
        (self.trivial_dest - self.start).directions()
    }
}

/// translation is given in original coordinates, so not rotated yet
#[derive(Default, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Transform {
    pub vec: Vec2<Local>,
    pub rotation: Rotation,
}

impl From<Vec2<Local>> for Transform {
    fn from(value: Vec2<Local>) -> Self {
        Self {
            vec: value,
            rotation: Rotation::Id,
        }
    }
}

impl From<Rotation> for Transform {
    fn from(value: Rotation) -> Self {
        Self {
            vec: Vec2::default(),
            rotation: value,
        }
    }
}

impl Transform {
    pub fn identity() -> Self {
        Default::default()
    }
    pub fn new(translation: Vec2<Local>, rotation: Rotation) -> Self {
        Self {
            vec: translation,
            rotation,
        }
    }

    pub fn chain(&self, next: Self) -> Self {
        Self {
            vec: self.vec + next.vec.rotate(self.rotation),
            rotation: self.rotation + next.rotation,
        }
    }

    /// self.chain(self.inverse()) == Self::identity()
    /// self.inverse(self.inverse()) == self
    pub fn inverse(&self) -> Self {
        Self {
            vec: (-self.vec).rotate(-self.rotation),
            rotation: -self.rotation,
        }
    }

    fn from_motor_action(motor: Option<MotorAction>) -> Self {
        match motor {
            Some(MotorAction::Step) => Self {
                vec: Vec2::new_front(1),
                rotation: Default::default(),
            },
            Some(MotorAction::TurnLeft) => Self {
                vec: Default::default(),
                rotation: Rotation::Left,
            },
            Some(MotorAction::TurnRight) => Self {
                vec: Default::default(),
                rotation: Rotation::Right,
            },
            Some(MotorAction::StepBack) => Self {
                vec: Vec2::new_back(1),
                rotation: Rotation::default(),
            },
            None => Default::default(),
        }
    }

    fn apply<A: PositionAnchor>(
        &self,
        pos: Position<A>,
        facing: Direction,
    ) -> (Position<A>, Direction) {
        (pos + self.vec.global(facing), facing + self.rotation)
    }

    fn apply_dir(&self, facing: Direction) -> Direction {
        facing + self.rotation
    }
}

#[derive(Clone, Copy, Debug)]
enum ArmAction {
    Stab,
}
#[derive(Clone, Copy, PartialEq, Debug)]
enum MotorAction {
    Step,
    StepBack,
    TurnRight,
    TurnLeft,
}
impl MotorAction {
    const ALL_AND_NOTHING: [Option<MotorAction>; 5] = [
        Some(Self::Step),
        Some(Self::StepBack),
        Some(Self::TurnRight),
        Some(Self::TurnLeft),
        None,
    ];
    async fn execute(&self, motor: &mut Motor) {
        match self {
            MotorAction::Step => motor.step_fw().await,
            MotorAction::StepBack => motor.step_bw().await,
            MotorAction::TurnRight => motor.turn_right().await,
            MotorAction::TurnLeft => motor.turn_left().await,
        }
    }
}

fn instincts<D: RadarSize>(
    arm: &Arm,
    motor: &Motor,
    radar_scan: &RadarScan<D>,
    time_stamp: Instant,
) -> MotorArmAction {
    let max_stab_wait = Duration::from_ticks(10_000);
    if arm.is_ready() {
        // overwrite next movement with urgent one
        if radar_scan.at(Vec2::new_front(1)).unwrap().is_bot() {
            MotorArmAction {
                motor: None,
                arm: Some(ArmAction::Stab),
                arm_timeout: time_stamp + max_stab_wait,
            }
        } else if radar_scan.at(Vec2::new_left(1)).unwrap().is_bot() {
            MotorArmAction {
                motor: Some(MotorAction::TurnLeft),
                arm: Some(ArmAction::Stab),
                arm_timeout: time_stamp + max_stab_wait,
            }
        } else if radar_scan.at(Vec2::new_right(1)).unwrap().is_bot() {
            MotorArmAction {
                motor: Some(MotorAction::TurnRight),
                arm: Some(ArmAction::Stab),
                arm_timeout: time_stamp + max_stab_wait,
            }
        } else if radar_scan
            .at(Vec2::new_front(2))
            .is_some_and(|t| t.is_bot())
            && radar_scan
                .at(Vec2::new_front(1))
                .is_some_and(|t| t.is_empty())
            && motor.is_ready()
        {
            MotorArmAction {
                motor: Some(MotorAction::Step),
                arm: Some(ArmAction::Stab),
                arm_timeout: time_stamp + max_stab_wait,
            }
        } else {
            MotorArmAction {
                motor: None,
                arm: None,
                arm_timeout: time_stamp + max_stab_wait,
            }
        }
    } else {
        MotorArmAction {
            motor: None,
            arm: None,
            arm_timeout: time_stamp + max_stab_wait,
        }
    }
}

#[derive(Clone, Debug)]
struct MotorArmAction {
    motor: Option<MotorAction>,
    arm: Option<ArmAction>,
    arm_timeout: Instant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum ExecutionResult {
    Success,
    ArmTimeout,
    RadarReady,
}

/// First execute motor action to move to a desired location, then execute arm action to stab.
/// If a new radar scan is ready, or at a certain timeout instant, it is canceled.
async fn execute_with_arm_timeout(
    radar: &mut Radar,
    motor: &mut Motor,
    arm: &mut Arm,
    action: &MotorArmAction,
) -> (Transform, ExecutionResult) {
    if let Some(motor_action) = action.motor
        && matches!(
            select(radar.wait(), motor_action.execute(motor)).await,
            Either::First(_)
        )
    {
        return (Transform::identity(), ExecutionResult::RadarReady);
    }
    (
        Transform::from_motor_action(action.motor),
        if action.arm.is_some() {
            match select3(radar.wait(), arm.stab(), Timer::at(action.arm_timeout)).await {
                Either3::First(()) => ExecutionResult::RadarReady,
                Either3::Second(()) => ExecutionResult::Success,
                Either3::Third(()) => ExecutionResult::ArmTimeout,
            }
        } else {
            ExecutionResult::Success
        },
    )
}

/// terrain evaluation, higher is better
fn terrain_eval_func(get_terrain: impl Fn(Vec2<Local>) -> Option<bool>) -> i8 {
    let mut eval = 0; // large is good

    // back against the wall, very important!
    if get_terrain(Vec2::new_back(1)).is_some_and(|b| !b) {
        eval += 8;
    } else if get_terrain(Vec2::new_back(2)).is_some_and(|b| !b) {
        eval += 4;
    }

    // left
    if get_terrain(Vec2::new_left(1)).is_some_and(|b| !b) {
        eval += 2;
    } else if get_terrain(Vec2::new_left(2)).is_some_and(|b| !b) {
        // an empty space between us and the wall is better, because it
        // is a very save space for us to stab stupid bots
        eval += 3;
    }

    // right
    if get_terrain(Vec2::new_right(1)).is_some_and(|b| !b) {
        eval += 2;
    } else if get_terrain(Vec2::new_right(2)).is_some_and(|b| !b) {
        // an empty space between us and the wall is better, because it
        // is a very save space for us to stab stupid bots
        eval += 3;
    }

    // front
    if get_terrain(Vec2::new_front(1)).is_some_and(|b| !b) {
        // even more important than back against the wall, we want to keep able to move instantly
        eval -= 10;
    } else if get_terrain(Vec2::new_front(2)).is_some_and(|b| !b) {
        // one free space in front is optimal, because then we don't have to move to stab
        // but at the end of the day it is not that important
        eval += 1;
    }

    // Some evaluations:
    // corridor blocked:         -2
    // completely open:           0
    // corridor movable:          4
    // next to wall back:         8
    // three-wide corridor:       8
    // two-wide corridor:         9
    // corner:                   10
    // next to corner:           11
    // dead end:                 12
    // optimum:                  15
    // ←↑↓→
    eval
}

fn bot_eval_func(dir: Vec2<Local>, stab: bool, back_against_wall: bool) -> (u8, bool) {
    const VALUES: [[u8; 7]; 7] = [
        [0, 0, 0, 1, 0, 0, 0],
        [0, 0, 0, 5, 0, 0, 0],
        [1, 2, 3, 13, 3, 2, 1],
        [2, 7, 17, 255, 17, 7, 2],
        [1, 2, 5, 26, 5, 2, 1],
        [0, 1, 2, 13, 2, 1, 0],
        [0, 0, 1, 4, 1, 0, 0],
    ];

    if stab && dir == Vec2::new_front(1) {
        // this bot will no longer exist
        (0, true)
    } else if dir.front().unsigned_abs() > 3 || dir.right().unsigned_abs() > 3 {
        // this bot is far away
        (0, false)
    } else if back_against_wall && dir.back() > 0 {
        (0, false)
    } else {
        // unconventional indexing (back, right) instead of (right, front) to understand VALUES
        // intuitively
        (
            VALUES[usize::try_from(dir.back() + 3).unwrap()]
                [usize::try_from(dir.right() + 3).unwrap()],
            false,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum NavigationEvaluation {
    StepGood,
    StepBad,
    NoStep(Rotation), // no movement, rotation to good direction is described by rotation
    NoInformation,
}

fn min_rotation(from: Direction, to: &[Direction]) -> Option<Rotation> {
    to.iter().map(|to| *to - from).min_by_key(|rot| match rot {
        Rotation::Id => 0,
        Rotation::Left => 1,
        Rotation::Right => 1,
        Rotation::Inverse => 2,
    })
}

fn movement(
    nav_state: &BotNavState,
    transform_from_scan: Transform,
    radar_scan: &RadarScan<impl RadarSize>,
    bots: &[Vec2<Local>],
    can_stab: bool,
    time_stamp: Instant,
) -> MotorArmAction {
    let max_stab_wait = Duration::from_ticks(10_000);
    let nav_dirs = nav_state.preferred_directions();

    let (motor, stab) = MotorAction::ALL_AND_NOTHING
        .into_iter()
        .filter_map(|movement| {
            let transform_movement = Transform::from_motor_action(movement);
            let transform_next_from_scan = transform_from_scan.chain(transform_movement);
            let next_location = transform_next_from_scan.vec;

            let possible = radar_scan
                .at(next_location)
                .is_some_and(|tile| tile.is_walkable_terrain());
            let back_against_wall = radar_scan
                .at(transform_next_from_scan.chain(Vec2::new_back(1).into()).vec)
                .is_some_and(|tile| !tile.is_walkable_terrain());
            if possible {
                // add evaluation for all bots
                let (bot_eval, stab) = bots
                    .iter()
                    .map(|&bot| {
                        bot_eval_func(
                            transform_next_from_scan.inverse().chain(bot.into()).vec,
                            can_stab,
                            back_against_wall,
                        )
                    })
                    .fold(
                        (0, false),
                        |(value_acc, stab_acc): (u8, _), (value, stab)| {
                            (value_acc.saturating_add(value), stab_acc || stab)
                        },
                    );

                // evaluation for being able to walk forward
                // let wall_eval = wall_eval_func(radar_scan, &t);

                let wall_eval = -terrain_eval_func(|vec| {
                    radar_scan
                        .at(transform_next_from_scan.chain(vec.into()).vec)
                        .map(|tile| tile.is_walkable_terrain())
                });

                let nav_eval = if nav_dirs.is_empty() {
                    NavigationEvaluation::NoInformation
                } else {
                    let movement_dirs = transform_movement
                        .vec
                        .global(nav_state.facing())
                        .directions();
                    if let &[movement_dir] = movement_dirs {
                        let good = nav_dirs.contains(&movement_dir);
                        // println!(
                        //     "nav {:?} {}",
                        //     movement_dir,
                        //     if good { "good" } else { "bad" }
                        // );
                        if good {
                            NavigationEvaluation::StepGood
                        } else {
                            NavigationEvaluation::StepBad
                        }
                    } else {
                        // unwrap: we checked not empty
                        let remaining_rotation = min_rotation(
                            transform_movement.apply_dir(nav_state.facing()),
                            nav_dirs,
                        )
                        .unwrap();
                        // println!(
                        //     "nav {:?} {:?}",
                        //     transform_movement.apply_dir(nav_state.facing()),
                        //     remaining_rotation,
                        // );
                        NavigationEvaluation::NoStep(remaining_rotation)
                    }
                };

                // long-term evaluation
                Some((movement, stab, bot_eval, wall_eval, nav_eval))
            } else {
                None
            }
        })
        .min_by_key(|&(movement, stab, bot_eval, wall_eval, nav_eval)| {
            value_function(
                movement,
                stab,
                bot_eval,
                wall_eval,
                bots.is_empty(),
                nav_eval,
                nav_state.position_known(),
            )
        })
        .map(|(movement, stab, _, _, _)| (movement, stab))
        .unwrap_or((None, false));
    MotorArmAction {
        motor,
        arm: if stab { Some(ArmAction::Stab) } else { None },
        arm_timeout: time_stamp + max_stab_wait,
    }
}

/// the action that minimized these values is selected
fn value_function(
    movement: Option<MotorAction>,
    _stab: bool,
    bot_eval: u8,
    wall_eval: i8,
    _bots_empty: bool,
    nav_eval: NavigationEvaluation,
    pos_known: bool,
) -> (u8, i8, i8, bool, bool) {
    if pos_known {
        (
            // disincentivise backward with bot score due to cooldown cost
            bot_eval
                + if movement == Some(MotorAction::StepBack) {
                    3
                } else {
                    0
                },
            match nav_eval {
                NavigationEvaluation::StepGood => -4,
                NavigationEvaluation::StepBad => 1,
                NavigationEvaluation::NoStep(Rotation::Id) => -3,
                NavigationEvaluation::NoStep(Rotation::Left) => -2,
                NavigationEvaluation::NoStep(Rotation::Right) => -2,
                NavigationEvaluation::NoStep(Rotation::Inverse) => -1,
                NavigationEvaluation::NoInformation => 0,
            },
            wall_eval,
            // prefer forward
            movement != Some(MotorAction::Step),
            // prefer no movement if there are bots, prefer movement if there are no bots
            // movement.is_some() != (bots.len() == 0),
            movement.is_some(),
        )
    } else {
        (
            // disincentivise backward with bot score due to cooldown cost
            bot_eval
                + if movement == Some(MotorAction::StepBack) {
                    3
                } else {
                    0
                },
            0, // nav info is not available
            0, // don't hide in corners
            // prefer forward
            movement != Some(MotorAction::Step),
            // prefer no movement if there are bots, prefer movement if there are no bots
            // movement.is_some() != (bots.len() == 0),
            movement.is_none(),
        )
    }
}

/// get a list of all bots (that have not yet been stabbed) in the scan area
fn get_bot_list<const MAX_N_BOTS: usize, D: RadarSize>(
    transform: Transform,
    has_stabbed: bool,
    radar_scan: &RadarScan<D>,
) -> Vec<Vec2<Local>, MAX_N_BOTS> {
    let stabbed_location = has_stabbed.then_some(transform.chain(Vec2::new_front(1).into()).vec);
    if let Some(stabbed_location) = stabbed_location {
        radar_scan
            .iter_tile(Tile::Bot)
            .filter(|&v| v != stabbed_location)
            .collect()
    } else {
        radar_scan.iter_tile(Tile::Bot).collect()
    }
}

/// Navigation, Position, and Orientation
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct BotNavState {
    facing: Direction,
    global_pos: Option<GlobalPos>,
    last_pos_synced: Option<GlobalPos>,
    navigation_section: Option<NavigationSection>,
}

impl BotNavState {
    fn new(facing: Direction) -> Self {
        Self {
            facing,
            global_pos: None,
            last_pos_synced: None,
            navigation_section: None,
        }
    }

    fn position_known(&self) -> bool {
        self.global_pos.is_some()
    }

    fn facing(&self) -> Direction {
        self.facing
    }

    /// update position and facing after the bot moved, and sync with navigation task
    async fn update_and_sync(&mut self, transform: Transform, sync: &DataSync) {
        // apply transform to modify pos and facing
        if let Some(old_pos) = self.global_pos {
            let new_pos;
            (new_pos, self.facing) = transform.apply(old_pos, self.facing);
            self.global_pos = Some(new_pos);
        } else {
            self.facing = transform.apply_dir(self.facing);
        }

        if let Some(global_pos) = self.global_pos {
            // send new position to navigator
            if self.last_pos_synced.is_none_or(|last| last != global_pos) {
                if sync.channel_position.is_full() {
                    sync.channel_position.clear();
                    sync.channel_position.send(global_pos).await;
                    // println!(">>p clear {}", global_pos);
                } else {
                    sync.channel_position.send(global_pos).await;
                    // println!(">>p {}", global_pos);
                }
                self.last_pos_synced = Some(global_pos);
            }
        }

        self.sync_navigation(sync);

        if let Some(global_pos) = self.global_pos {
            // update navigation section
            NavigationSection::update(&mut self.navigation_section, global_pos);
        }
    }

    /// update global position if new scan is available to analyse
    fn analyse_scan(&mut self, radar_scan: &RadarScan<D7>) {
        if let Some(pos) = get_global_pos(&MapSection::from_scan(radar_scan, self.facing))
            && self.global_pos.is_none_or(|old_pos| old_pos != pos)
        {
            self.global_pos = Some(pos);
            println!("pos update {}", pos);
        }
    }

    /// get latest navigation update
    fn sync_navigation(&mut self, sync: &DataSync) {
        if let Some(update) = sync.signal_navigation.try_take() {
            match update {
                Ok(section) => {
                    self.navigation_section = Some(section);
                    // println!("<<n {} {}", section.start, section.trivial_dest);
                }
                Err(err) => println!("nav err: {:?}", err),
            }
        }
    }

    /// navigation directions
    fn preferred_directions(&self) -> &[Direction] {
        self.navigation_section
            .filter(|section| Some(section.start) == self.global_pos)
            .map(|section| section.directions())
            .unwrap_or(&[])
    }
}

#[task]
async fn foreground(
    mut bot: Bot,
    sync: DataSync,
    signal_complete: &'static Signal<NoopRawMutex, ()>,
) -> ! {
    // settings
    const MAX_N_BOTS: usize = 28;

    let radar = &mut bot.radar;
    let arm = &mut bot.arm;
    let motor = &mut bot.motor;

    let destination = pos_east_south(14, 36);
    sync.signal_destination.signal(destination);
    // println!("-> dest {}", destination);
    println!("destination: {}", destination);

    let mut nav_state = BotNavState::new(bot.compass.try_direction().unwrap());

    'main_loop: loop {
        let radar_scan = &radar.scan::<D7>().await;
        let radar_timestamp = Instant::now();

        let action = instincts(arm, motor, radar_scan, radar_timestamp);
        let (transform, execution_result) =
            execute_with_arm_timeout(radar, motor, arm, &action).await;

        nav_state.analyse_scan(radar_scan);
        nav_state.update_and_sync(transform, &sync).await;

        if nav_state.global_pos == Some(destination) {
            println!("-- done --");
            signal_complete.signal(());
            Timer::after_secs(2).await;
            exit();
        }

        if execution_result == ExecutionResult::RadarReady {
            // new information is available and should be used
            continue 'main_loop;
        } else if action.motor.is_some() {
            // keep radar and motor in sync by not performing any more movements
            continue 'main_loop;
        } else {
            let bots = get_bot_list::<MAX_N_BOTS, _>(transform, action.arm.is_some(), radar_scan);

            nav_state.sync_navigation(&sync);
            let action = movement(
                &nav_state,
                transform,
                radar_scan,
                &bots,
                arm.is_ready(),
                radar_timestamp,
            );
            let (transform, _result) = execute_with_arm_timeout(radar, motor, arm, &action).await;

            nav_state.update_and_sync(transform, &sync).await;
        }
    }
}

#[derive(Clone)]
struct DataSync {
    channel_position: &'static Channel<NoopRawMutex, GlobalPos, 16>,
    signal_navigation: &'static Signal<NoopRawMutex, Result<NavigationSection, NavigatorError>>,
    signal_destination: &'static Signal<NoopRawMutex, GlobalPos>,
    signal_reset: &'static Signal<NoopRawMutex, ()>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum SyncReceived {
    Start(GlobalPos),
    Destination(GlobalPos),
    Reset,
}

impl DataSync {
    async fn receive_next(&self) -> SyncReceived {
        match select3(
            self.channel_position.receive(),
            self.signal_reset.wait(),
            self.signal_destination.wait(),
        )
        .await
        {
            Either3::First(start) => SyncReceived::Start(start),
            Either3::Second(()) => SyncReceived::Reset,
            Either3::Third(destination) => SyncReceived::Destination(destination),
        }
    }

    fn try_receive_next(&self) -> Option<SyncReceived> {
        if self.signal_reset.try_take().is_some() {
            Some(SyncReceived::Reset)
        } else if let Some(destination) = self.signal_destination.try_take() {
            Some(SyncReceived::Destination(destination))
        } else if let Ok(start) = self.channel_position.try_receive() {
            Some(SyncReceived::Start(start))
        } else {
            None
        }
    }
}

/// commands to send to the navigator:
/// - set destination
/// - set start (channel)
/// - reset
#[task]
async fn navigation(sync: DataSync) -> ! {
    let sync = &sync;
    let mut nav = NavigatorEnum::New(Navigator::new(navigator_resources()));

    loop {
        nav = match nav {
            NavigatorEnum::New(nav) => {
                // println!("new");
                async move {
                    match sync.receive_next().await {
                        SyncReceived::Start(start) => nav.set_start(start).into(),
                        SyncReceived::Reset => nav.into(),
                        SyncReceived::Destination(destination) => {
                            nav.set_destination(destination).into()
                        }
                    }
                }
                .await
            }
            NavigatorEnum::OnlyStart(nav) => {
                // println!("only start");
                async move {
                    match sync.receive_next().await {
                        SyncReceived::Start(start) => nav.set_start(start).into(),
                        SyncReceived::Destination(destination) => {
                            nav.set_destination(destination).into()
                        }
                        SyncReceived::Reset => nav.reset().into(),
                    }
                }
                .await
            }
            NavigatorEnum::OnlyDestination(nav) => {
                // println!("only dest");
                async move {
                    match sync.receive_next().await {
                        SyncReceived::Start(start) => nav.set_start(start).into(),
                        SyncReceived::Destination(destination) => {
                            nav.set_destination(destination).into()
                        }
                        SyncReceived::Reset => nav.reset().into(),
                    }
                }
                .await
            }
            NavigatorEnum::Initialized(nav) => {
                // println!("init");
                async move {
                    match sync.try_receive_next() {
                        Some(SyncReceived::Start(start)) => nav.set_start(start).into(),
                        Some(SyncReceived::Destination(destination)) => {
                            nav.set_destination(destination).into()
                        }
                        Some(SyncReceived::Reset) => nav.reset().into(),
                        None => nav.compute().await.into(),
                    }
                }
                .await
            }
            NavigatorEnum::Ready(nav) => {
                // println!("ready");
                async move {
                    sync.signal_navigation.signal(Ok(NavigationSection {
                        start: nav.get_start(),
                        trivial_dest: nav.next_trivial_target(),
                    }));
                    match sync.receive_next().await {
                        SyncReceived::Start(start) => nav.set_start(start).into(),
                        SyncReceived::Destination(destination) => {
                            nav.set_destination(destination).into()
                        }
                        SyncReceived::Reset => nav.reset().into(),
                    }
                }
                .await
            }
            NavigatorEnum::Failed(nav) => {
                // println!("failed");
                async move {
                    sync.signal_navigation.signal(Err(nav.get_error()));
                    match sync.receive_next().await {
                        SyncReceived::Start(start) => nav.set_start(start).into(),
                        SyncReceived::Destination(destination) => {
                            nav.set_destination(destination).into()
                        }
                        SyncReceived::Reset => nav.reset().into(),
                    }
                }
                .await
            }
            NavigatorEnum::UpdateFailed(nav) => {
                // println!("update failed");
                async move {
                    sync.signal_navigation.signal(Err(nav.get_error()));
                    match sync.receive_next().await {
                        SyncReceived::Start(start) => nav.set_start(start).into(),
                        SyncReceived::Destination(destination) => {
                            nav.set_destination(destination).into()
                        }
                        SyncReceived::Reset => nav.reset().into(),
                    }
                }
                .await
            }
            NavigatorEnum::UpdateScheduled(nav) => {
                // println!("update scheduled");
                async move {
                    match sync.try_receive_next() {
                        Some(SyncReceived::Start(start)) => nav.set_start(start).into(),
                        Some(SyncReceived::Destination(destination)) => {
                            nav.set_destination(destination).into()
                        }
                        Some(SyncReceived::Reset) => nav.reset().into(),
                        None => nav.compute().await.into(),
                    }
                }
                .await
            }
            NavigatorEnum::Invalid => unreachable!(),
        }
    }
}

#[task]
async fn watchdog(signal_complete: &'static Signal<NoopRawMutex, ()>) -> ! {
    let mut dog = StatsDog::<KartoffelClock>::new();
    loop {
        dog.restart_timer();
        Breakpoint::new().await;
        let _elapsed = dog.feed();

        // if _elapsed > Duration::from_ticks(32_000) {
        //     println!("W: blocked {}", elapsed);
        // }

        if signal_complete.try_take().is_some() {
            println!("{}", dog);
        }
    }
}
