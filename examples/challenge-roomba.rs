#![no_main]
#![no_std]

use alloc::boxed::Box;
use async_kartoffel::{
    algorithm::{
        dist_walk_with_rotation, Breakpoint, ChunkMap, ChunkTerrain, DistBotWalk, DistManhattan,
        DistanceMeasure, Exploration, Map, Navigation, Terrain,
    },
    print, println, Arm, Bot, Direction, Distance, Duration, Instant, Local, Motor, Position,
    Radar, RadarScan, RadarScanWeak, RadarSize, Rotation, Tile, Timer, D9,
};
use core::num::NonZeroU16;
use core::ops::DerefMut;
use embassy_executor::{task, Executor};
use embassy_futures::select::{select, select3, Either, Either3};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, signal::Signal};
use heapless::{FnvIndexMap, Vec};
use static_cell::StaticCell;

extern crate alloc;

#[no_mangle]
fn main() {
    static EXECUTOR: StaticCell<Executor> = StaticCell::new();
    static SIGNAL_MAP: StaticCell<Signal<NoopRawMutex, MapUpdate>> = StaticCell::new();
    static SIGNAL_NAVIGATION: StaticCell<Signal<NoopRawMutex, Position>> = StaticCell::new();
    static SIGNAL_COMPLETE: StaticCell<Signal<NoopRawMutex, ()>> = StaticCell::new();

    let executor = EXECUTOR.init(Executor::new());
    let signal_map = SIGNAL_MAP.init(Signal::new());
    let signal_navigation = SIGNAL_NAVIGATION.init(Signal::new());
    let signal_complete = SIGNAL_COMPLETE.init(Signal::new());

    println!("async_kartoffel");

    executor.run(|spawner| {
        spawner
            .spawn(foreground(Bot::take(), signal_map, signal_navigation))
            .unwrap();
        spawner
            .spawn(map(signal_map, signal_navigation, signal_complete))
            .unwrap();
        spawner.spawn(watchdog(signal_complete)).unwrap();
    })
}

/// translation is applied before rotation, so still in original coordinates
#[derive(Default)]
struct Transform {
    translation: Distance<Local>,
    rotation: Rotation,
}

impl Transform {
    fn transform(&self, vec: Distance<Local>) -> Distance<Local> {
        (vec + self.translation).rotate(self.rotation)
    }
    fn transform_rot(&self, rot: Rotation) -> Rotation {
        rot + self.rotation
    }

    fn from_motor_action(motor: Option<MotorAction>) -> Self {
        match motor {
            Some(MotorAction::Step) => Self {
                translation: Distance::new_front(1),
                rotation: Default::default(),
            },
            Some(MotorAction::TurnLeft) => Self {
                translation: Default::default(),
                rotation: Rotation::Left,
            },
            Some(MotorAction::TurnRight) => Self {
                translation: Default::default(),
                rotation: Rotation::Right,
            },
            None => Default::default(),
        }
    }
}

#[derive(Clone, Copy)]
enum ArmAction {
    #[allow(unused)]
    Stab,
    Pick,
}
impl ArmAction {
    async fn execute(&self, arm: &mut Arm) {
        match self {
            ArmAction::Stab => arm.stab().await,
            ArmAction::Pick => arm.pick().await,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Debug)]
enum MotorAction {
    Step,
    TurnRight,
    TurnLeft,
}
impl MotorAction {
    const ALL_AND_NOTHING: [Option<MotorAction>; 4] = [
        Some(Self::Step),
        Some(Self::TurnRight),
        Some(Self::TurnLeft),
        None,
    ];
    async fn execute(&self, motor: &mut Motor) {
        match self {
            MotorAction::Step => motor.step().await,
            MotorAction::TurnRight => motor.turn_right().await,
            MotorAction::TurnLeft => motor.turn_left().await,
        }
    }
}
struct MotorArmAction {
    motor: Option<MotorAction>,
    arm: Option<ArmAction>,
    arm_timeout: Instant,
}

struct MapUpdate {
    scan: RadarScanWeak<D9>,
    pos: Position,
    direction: Direction,
}

fn instincts<D: RadarSize>(
    arm: &Arm,
    _motor: &Motor,
    radar_scan: &RadarScan<D>,
    time_stamp: Instant,
) -> MotorArmAction {
    let max_stab_wait = Duration::from_ticks(10_000);
    if arm.is_ready() {
        // overwrite next movement with urgent one
        if radar_scan.at(Distance::new_front(1)).unwrap() == Tile::Flag {
            MotorArmAction {
                motor: None,
                arm: Some(ArmAction::Pick),
                arm_timeout: time_stamp + Duration::from_secs(10),
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

async fn execute_with_arm_timeout(
    radar: &mut Radar,
    motor: &mut Motor,
    arm: &mut Arm,
    action: MotorArmAction,
    position: &mut Position,
    direction: &mut Direction,
) -> bool {
    if let Some(motor_action) = action.motor {
        if matches!(
            select(radar.wait(), motor_action.execute(motor)).await,
            Either::First(_)
        ) {
            return false;
        } else {
            match motor_action {
                MotorAction::TurnLeft => *direction += Rotation::Left,
                MotorAction::TurnRight => *direction += Rotation::Right,
                MotorAction::Step => *position += Distance::new_front(1).global(*direction),
            }
        }
    }
    if let Some(arm_action) = action.arm {
        if matches!(
            select3(
                radar.wait(),
                arm_action.execute(arm),
                Timer::at(action.arm_timeout),
            )
            .await,
            Either3::First(_)
        ) {
            return false;
        }
    }
    true
}

fn movement(
    pos: Position,
    direction: Direction,
    radar_scan: &RadarScan<impl RadarSize>,
    navigation_target: &Option<Position>,
    time_stamp: Instant,
) -> MotorArmAction {
    let max_stab_wait = Duration::from_ticks(10_000);
    let (motor, arm) = MotorAction::ALL_AND_NOTHING
        .into_iter()
        .filter_map(|movement| {
            let t = Transform::from_motor_action(movement);
            let next_location = t.transform(Default::default());
            let next_rotation = t.transform_rot(Rotation::Id);
            let possible = next_location == Default::default()
                || radar_scan
                    .at(next_location)
                    .is_some_and(|tile| tile.is_empty());
            if possible {
                // long-term evaluation
                let eval = navigation_target.as_ref().map_or(0, |&target| {
                    let pos_next = pos + next_location.global(direction);
                    let ori_next = direction + next_rotation;
                    dist_walk_with_rotation(target - pos_next, ori_next)
                });
                Some((movement, eval))
            } else {
                None
            }
        })
        .min_by_key(|&(movement, eval)| {
            // this is the values that are minimized, in that order
            // TODO which order is best?
            (
                eval,
                movement.is_some(),                  // prefer no movement
                movement == Some(MotorAction::Step), // prefer not moving forward
            )
        })
        .map(|(movement, _eval)| (movement, None))
        .unwrap_or((None, None));
    MotorArmAction {
        motor,
        arm,
        arm_timeout: time_stamp + max_stab_wait,
    }
}

// https://en.wikipedia.org/wiki/Methods_of_computing_square_roots
fn isqrt(n: u64) -> u64 {
    // X_(n+1)
    let mut x = n;

    // c_n
    let mut c = 0;

    // d_n which starts at the highest power of four <= n
    let mut d = 1u64 << 30; // The second-to-top bit is set.
                            // Same as ((unsigned) INT32_MAX + 1) / 2.
    while d > n {
        d >>= 2;
    }

    // for dₙ … d₀
    while d != 0 {
        if x >= c + d {
            // if X_(m+1) ≥ Y_m then a_m = 2^m
            x -= c + d; // X_m = X_(m+1) - Y_m
            c = (c >> 1) + d; // c_(m-1) = c_m/2 + d_m (a_m is 2^m)
        } else {
            c >>= 1; // c_(m-1) = c_m/2      (aₘ is 0)
        }
        d >>= 2; // d_(m-1) = d_m/4
    }
    c // c_(-1)
}

#[task]
async fn watchdog(signal_complete: &'static Signal<NoopRawMutex, ()>) -> ! {
    let mut sum_duration = Duration::default();
    let mut counter = 0;
    let mut max_duration = Duration::default();
    let mut min_duration = Duration::default();
    let mut sum_sq_duration = 0u64;

    let mut last_time = Instant::now();
    loop {
        Breakpoint::new().await;
        let now = Instant::now();
        let duration = (now - last_time).unwrap();

        sum_duration += duration;
        counter += 1;
        max_duration = max_duration.max(duration);
        min_duration = min_duration.min(duration);
        sum_sq_duration += u64::from(duration.as_ticks()).pow(2);

        if signal_complete.try_take().is_some() {
            println!("watchdog blocked");
            println!("n   : {}", counter);
            println!("min : {}", min_duration.as_ticks());
            println!("mean: {}", sum_duration.as_ticks() / counter);
            println!("max : {}", max_duration.as_ticks());
            // std = 1/(N-1) * sum((x - µ)^2)
            //     = 1/(N-1) * (sum(x^2) - 2 sum(x) µ + N µ^2)
            //     = (sum(x^2) - N µ^2) / (N-1)
            let std = isqrt(
                (sum_sq_duration - u64::from(sum_duration.as_ticks()).pow(2) / u64::from(counter))
                    / (u64::from(counter) - 1),
            );
            println!("std : {}", std);
        }

        last_time = Instant::now();
    }
}

#[task]
async fn foreground(
    mut bot: Bot,
    signal_map: &'static Signal<NoopRawMutex, MapUpdate>,
    signal_nav: &'static Signal<NoopRawMutex, Position>,
) -> ! {
    let radar = &mut bot.radar;
    let arm = &mut bot.arm;
    let motor = &mut bot.motor;

    let mut pos = Position::default();

    let mut direction = bot.compass.try_direction().unwrap();

    let mut nav_target: Option<Position> = None;

    'main_loop: loop {
        let radar_scan = &radar.scan::<D9>().await;
        let radar_timestamp = Instant::now();
        signal_map.signal(MapUpdate {
            scan: radar_scan.weak(),
            pos,
            direction,
        });

        let action = instincts(arm, motor, radar_scan, radar_timestamp);
        if !execute_with_arm_timeout(radar, motor, arm, action, &mut pos, &mut direction).await {
            continue 'main_loop;
        }

        // update navigation target, if background task has provided a new update
        if let Some(target) = signal_nav.try_take() {
            nav_target = Some(target);
        }

        let action = movement(pos, direction, radar_scan, &nav_target, radar_timestamp);
        if !execute_with_arm_timeout(radar, motor, arm, action, &mut pos, &mut direction).await {
            continue 'main_loop;
        }
    }
}

fn get_next<const N: usize, T: Map<Option<NonZeroU16>>>(
    nav: &Navigation<T, N>,
    pos: Position,
) -> Position {
    let valid_next = nav.next_step(pos);
    if valid_next.east {
        pos + Distance::new_east(1)
    } else if valid_next.west {
        pos + Distance::new_west(1)
    } else if valid_next.north {
        pos + Distance::new_north(1)
    } else if valid_next.south {
        pos + Distance::new_south(1)
    } else {
        pos
    }
}

fn print_map<T: Map<Terrain>, const N: usize>(
    map: &T,
    pos: Position,
    dist: u16,
    markers: &FnvIndexMap<Position, char, N>,
) {
    let dist = dist as i16;
    for south in -dist..dist {
        for east in -dist..dist {
            let pos_print = pos + Distance::new_global(east, -south);
            let ch = match markers.get(&pos_print) {
                Some(&ch) => ch,
                None => match map.get(pos_print) {
                    Some(Terrain::Reachable) => ' ',
                    Some(Terrain::Walkable) => '~',
                    Some(Terrain::Blocked) => '█',
                    _ => '░',
                },
            };
            print!("{}", ch);
        }
        println!("");
    }
}

type MyMap = ChunkMap<128, Terrain, ChunkTerrain>;
type MyNav = Navigation<ChunkMap<64, Option<NonZeroU16>, [[Option<NonZeroU16>; 8]; 8]>, 64>;
type MyExp = Exploration<256, MyMap>;

#[task]
async fn map(
    signal_map: &'static Signal<NoopRawMutex, MapUpdate>,
    signal_nav: &'static Signal<NoopRawMutex, Position>,
    signal_complete: &'static Signal<NoopRawMutex, ()>,
) -> ! {
    let mut map: Box<MyMap> = Default::default();
    map.set(Default::default(), Terrain::Walkable).unwrap();
    let mut computations: Computations = Default::default();
    println!("alloc successfull");
    computations
        .exploration
        .initialize(map.deref_mut(), Default::default());

    let mut flags = Vec::<Position, 4>::new();

    let mut last_update: Option<MapUpdate> = None;

    loop {
        // wait for scan (if not already saved)
        let MapUpdate {
            scan,
            pos,
            direction,
        } = match last_update.take() {
            Some(update) => update,
            None => signal_map.wait().await,
        };

        // update map
        if let Some(radar_scan) = scan.upgrade() {
            if let Err(err) = map.update(&radar_scan, pos, direction) {
                println!("error in map {:?}", err);
            }
            if let Err(err) = computations.exploration.activate(pos, &radar_scan) {
                println!("error in exploration {:?}", err);
            }

            // keep only flags that are not updated by this scan
            flags = flags
                .into_iter()
                .filter(|&flag_pos| !radar_scan.contains((flag_pos - pos).local(direction)))
                .collect();
            for dist in radar_scan.iter_tile(Tile::Flag) {
                let flag_pos = pos + dist.global(direction);
                flags.push(flag_pos).expect("more than 4 flags found");
            }
        }
        Breakpoint::new().await;

        let results = computations
            .run(pos, map.deref_mut(), &flags, signal_complete)
            .await;
        if let Some(eval) = results {
            signal_nav.signal(eval);
        }
    }
}

#[derive(Default)]
struct Computations {
    nav: Box<MyNav>,
    exploration: Box<MyExp>,
    target: Option<Position>,
    exploration_completed: bool,
}

impl Computations {
    async fn run(
        &mut self,
        pos: Position,
        map: &mut ChunkMap<128, Terrain, ChunkTerrain>,
        flags: &[Position],
        signal_complete: &'static Signal<NoopRawMutex, ()>,
    ) -> Option<Position> {
        self.exploration.run(map).await;
        if self.exploration.get_state().is_complete() && !self.exploration_completed {
            println!("map complete");
            self.exploration_completed = true;
            signal_complete.signal(());
        }

        // reset target if reached
        if self.target == Some(pos) {
            self.target = None
        }
        // flags are priority targets
        if self.target.is_none_or(|target| !flags.contains(&target)) {
            if let Some(&target_flag) = flags
                .into_iter()
                .filter(|&&flag_pos| {
                    map.get(flag_pos)
                        .is_some_and(|terrain| terrain == Terrain::Reachable)
                })
                .min_by_key(|&&flag_pos| DistManhattan::measure(flag_pos - pos))
            {
                self.target = Some(target_flag);
                self.nav.initialize(pos, target_flag);
                print_map(
                    map,
                    pos,
                    8,
                    &FnvIndexMap::<_, _, 2>::from_iter([(pos, '@'), (target_flag, '=')]),
                );
            }
        }
        // target at border of known reachable
        if self.target.is_none() {
            if let Some(unknown_reachables) = self.exploration.border(map) {
                self.target = unknown_reachables
                    .min_by_key(|&pos_border| DistBotWalk::measure(pos_border - pos));
                if let Some(target) = self.target {
                    self.nav.initialize(pos, target);
                }
            }
        }

        if let Some(task) = self.nav.get_state().task() {
            if task.from != pos {
                self.nav.update_start(pos).unwrap();
            }
        }

        Breakpoint::new().await;

        self.nav
            .run(|pos| map.get(pos).is_some_and(|t| t.is_known_walkable()))
            .await;

        if self.nav.get_state().is_success() {
            Some(get_next(&self.nav, pos))
        } else {
            None
        }
    }
}
