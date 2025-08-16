#![no_main]
#![no_std]
#![feature(custom_test_frameworks)]
#![test_runner(test_kartoffel::runner)]
#![feature(iter_next_chunk)]

use alloc::boxed::Box;

use alloc::string::ToString;
use async_algorithm::{
    Breakpoint, ChunkMapHash, ChunkTerrain, DistanceBotWalk, DistanceMeasure, Exploration, Map,
    Navigation, StatsDog, Terrain, distance_walk_with_rotation, update_chunk_map,
};
use async_kartoffel::Duration;
use async_kartoffel::{
    Arm, Bot, Instant, KartoffelClock, Motor, Radar, RadarScan, RadarScanWeak, Timer, print,
    println,
};
use async_kartoffel_generic::{
    D3, Direction, Local, Position, RadarScanTrait, RadarSize, Rotation, Tile, Vec2,
};
use core::num::NonZeroU16;
use core::ops::Deref;
use core::ops::DerefMut;
use core::ops::RangeInclusive;
use embassy_executor::{Executor, task};
use embassy_futures::select::{Either, Either3, select, select3};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, signal::Signal};
use heapless::Vec;
use static_cell::StaticCell;

extern crate alloc;

struct DropTimer<'a> {
    init: Instant,
    name: &'a str,
}
impl<'a> DropTimer<'a> {
    fn new(name: &'a str) -> Self {
        Self {
            init: Instant::now(),
            name,
        }
    }
}
impl Drop for DropTimer<'_> {
    fn drop(&mut self) {
        println!("{}: {}", self.name, (Instant::now() - self.init).unwrap());
    }
}

#[unsafe(no_mangle)]
fn main() {
    static EXECUTOR: StaticCell<Executor> = StaticCell::new();
    static BOT: StaticCell<Bot> = StaticCell::new();
    static SIGNAL_MAP: StaticCell<Signal<NoopRawMutex, MapUpdate>> = StaticCell::new();
    static SIGNAL_NAVIGATION: StaticCell<Signal<NoopRawMutex, NavigationEvaluationN<3>>> =
        StaticCell::new();

    let executor = EXECUTOR.init(Executor::new());
    let bot = BOT.init(Bot::take());
    let signal_map = SIGNAL_MAP.init(Signal::new());
    let signal_navigation = SIGNAL_NAVIGATION.init(Signal::new());

    let map: Box<MyMap> = Default::default();
    let nav: Box<MyNav> = Default::default();
    let exploration: Box<MyExp> = Default::default();

    println!("async_kartoffel");
    println!("explorer");

    executor.run(|spawner| {
        spawner
            .spawn(foreground(bot, signal_map, signal_navigation))
            .unwrap();
        spawner
            .spawn(background(
                map,
                nav,
                exploration,
                signal_map,
                signal_navigation,
            ))
            .unwrap();
        spawner.spawn(watchdog()).unwrap();
    })
}

pub trait Evaluation {
    /// smaller is better
    fn get_at(&self, pos: Position, direction: Direction) -> u8;
}

// #[derive(Clone)]
// struct NavigationEvaluation {
//     next_destination: Position,
// }
// impl NavigationEvaluation {
//     fn new(next_destination: Position) -> Self {
//         Self { next_destination }
//     }
// }
// impl Evaluation for NavigationEvaluation {
//     fn get_at(&self, pos: Position, direction: Direction) -> u8 {
//         distance_walk_with_rotation(self.next_destination - pos, direction) as u8
//     }
// }

#[derive(Clone)]
struct NavigationEvaluationN<const N: usize> {
    waypoints: Vec<Position, N>,
    range: u16,
}
impl<const N: usize> NavigationEvaluationN<N> {
    fn new(waypoints: Vec<Position, N>, range: u16) -> Self {
        Self { waypoints, range }
    }
}
impl<const N: usize> Evaluation for NavigationEvaluationN<N> {
    fn get_at(&self, pos: Position, direction: Direction) -> u8 {
        for &waypoint in self.waypoints.iter().rev() {
            if DistanceBotWalk::measure(waypoint - pos) < self.range {
                return distance_walk_with_rotation(waypoint - pos, direction) as u8;
            }
        }
        u8::MAX
    }
}

/// translation is given in original coordinates, so not rotated yet
#[derive(Default)]
pub struct Transform {
    translation: Vec2<Local>,
    rotation: Rotation,
}

impl Transform {
    pub fn transform(&self, vec: Vec2<Local>) -> Vec2<Local> {
        (vec + self.translation).rotate(self.rotation)
    }
    pub fn transform_rot(&self, rot: Rotation) -> Rotation {
        rot + self.rotation
    }

    pub fn inverse_transform(&self, vec: Vec2<Local>) -> Vec2<Local> {
        vec.rotate(-self.rotation) - self.translation
    }
    pub fn inverse_transform_rot(&self, rot: Rotation) -> Rotation {
        rot - self.rotation
    }

    fn from_motor_action(motor: Option<MotorAction>) -> Self {
        match motor {
            Some(MotorAction::Step) => Self {
                translation: Vec2::new_front(1),
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
    Stab,
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
            MotorAction::Step => motor.step_fw().await,
            MotorAction::TurnRight => motor.turn_right().await,
            MotorAction::TurnLeft => motor.turn_left().await,
        }
    }
}

struct MapUpdate {
    scan: RadarScanWeak<D3>,
    scan_pos: Position,
    direction: Direction,
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

struct MotorArmAction {
    motor: Option<MotorAction>,
    arm: Option<ArmAction>,
    arm_timeout: Instant,
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
                MotorAction::Step => *position += Vec2::new_front(1).global(*direction),
            }
        }
    }
    if action.arm.is_some()
        && matches!(
            select3(radar.wait(), arm.stab(), Timer::at(action.arm_timeout),).await,
            Either3::First(_)
        )
    {
        return false;
    }
    true
}

fn bot_eval_func(dir: Vec2<Local>, stab: bool) -> (u8, bool) {
    const VALUES: [[u8; 7]; 7] = [
        [0, 0, 0, 1, 0, 0, 0],
        [0, 0, 1, 4, 1, 0, 0],
        [0, 1, 4, 16, 4, 1, 0],
        [1, 5, 12, 255, 12, 5, 1],
        [0, 1, 4, 16, 4, 1, 0],
        [0, 0, 1, 8, 1, 0, 0],
        [0, 0, 0, 2, 0, 0, 0],
    ];

    if stab && dir == Vec2::new_front(1) {
        // this bot will no longer exist
        (0, true)
    } else if dir.front().unsigned_abs() > 3 || dir.right().unsigned_abs() > 3 {
        // this bot is far away
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

fn wall_eval_func<D: RadarSize>(radar_scan: &RadarScan<D>, transform: &Transform) -> u8 {
    let front = transform.transform(Vec2::new_front(1));
    match radar_scan.at(front) {
        Some(tile) if tile.is_walkable_terrain() => 0,
        Some(_) => 2,
        None => 1,
    }
}

fn movement(
    pos: Position,
    direction: Direction,
    radar_scan: &RadarScan<impl RadarSize>,
    bots: &[Vec2<Local>],
    can_stab: bool,
    long_term_eval: &Option<impl Evaluation>,
    time_stamp: Instant,
) -> MotorArmAction {
    let max_stab_wait = Duration::from_ticks(10_000);
    let (motor, stab) = MotorAction::ALL_AND_NOTHING
        .into_iter()
        .filter_map(|movement| {
            let t = Transform::from_motor_action(movement);
            let next_location = t.transform(Default::default());
            let next_rotation = t.transform_rot(Rotation::Id);
            let possible = radar_scan
                .at(next_location)
                .is_some_and(|tile| tile.is_walkable_terrain());
            if possible {
                // add evaluation for all bots
                let (bot_eval, stab) = bots
                    .iter()
                    .map(|vec| bot_eval_func(t.inverse_transform(*vec), can_stab))
                    .fold(
                        (0, false),
                        |(value_acc, stab_acc): (u8, _), (value, stab)| {
                            (value_acc.saturating_add(value), stab_acc || stab)
                        },
                    );

                // evaluation for being able to walk forward
                let wall_eval = wall_eval_func(radar_scan, &t);

                // long-term evaluation
                let long_eval = long_term_eval.as_ref().map_or(0, |eval| {
                    let pos_next = pos + next_location.global(direction);
                    let ori_next = direction + next_rotation;
                    eval.get_at(pos_next, ori_next)
                });
                Some((movement, stab, bot_eval, long_eval, wall_eval))
            } else {
                None
            }
        })
        .min_by_key(|&(movement, _stab, bot_eval, long_eval, _wall_eval)| {
            // this is the values that are minimized, in that order
            // TODO which order is best?
            (
                bot_eval,
                long_eval,
                // wall_eval,
                movement.is_none(),
                movement != Some(MotorAction::Step),
                // movement.is_some(),
                // movement == Some(MotorAction::Step),
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

#[task]
async fn foreground(
    bot: &'static mut Bot,
    signal_map: &'static Signal<NoopRawMutex, MapUpdate>,
    signal_nav: &'static Signal<NoopRawMutex, NavigationEvaluationN<3>>,
) -> ! {
    // settings
    const MAX_N_BOTS: usize = 24;

    let radar = &mut bot.radar;
    let arm = &mut bot.arm;
    let motor = &mut bot.motor;

    let mut pos = Position::default();

    let mut direction = bot.compass.try_direction().unwrap();

    let mut nav_eval: Option<NavigationEvaluationN<3>> = None;

    'main_loop: loop {
        let radar_scan = &radar.scan::<D3>().await;
        let radar_timestamp = Instant::now();
        signal_map.signal(MapUpdate {
            scan: radar_scan.weak(),
            scan_pos: pos,
            direction,
        });

        let action = instincts(arm, motor, radar_scan, radar_timestamp);
        if !execute_with_arm_timeout(radar, motor, arm, action, &mut pos, &mut direction).await {
            continue 'main_loop;
        }

        // update long term evaluation function, if background task has provided a new update
        if let Some(eval) = signal_nav.try_take() {
            nav_eval = Some(eval.clone());
        }

        let mut bots = Vec::<_, MAX_N_BOTS>::new();
        for bot in radar_scan.iter_tile(Tile::Bot) {
            bots.push(bot).unwrap();
        }
        let can_stab = arm.is_ready();
        let action = movement(
            pos,
            direction,
            radar_scan,
            &bots,
            can_stab,
            &nav_eval,
            radar_timestamp,
        );
        if !execute_with_arm_timeout(radar, motor, arm, action, &mut pos, &mut direction).await {
            continue 'main_loop;
        }
    }
}

#[task]
async fn background(
    mut map: Box<MyMap>,
    mut nav: Box<MyNav>,
    mut exploration: Box<MyExp>,
    signal_map: &'static Signal<NoopRawMutex, MapUpdate>,
    signal_nav: &'static Signal<NoopRawMutex, NavigationEvaluationN<3>>,
) -> ! {
    map.set(Default::default(), Terrain::Walkable).unwrap();
    exploration.initialize(&mut map, Default::default());

    let mut destination: Option<Position> = None;
    let mut exploration_completed = false;
    let mut last_update: Option<MapUpdate> = None;

    loop {
        // wait for scan (if not already saved)
        let MapUpdate {
            scan,
            scan_pos,
            direction,
        } = match last_update.take() {
            Some(update) => update,
            None => signal_map.wait().await,
        };

        // update map
        {
            // let _t = DropTimer::new("t000");
            if let Some(radar_scan) = scan.upgrade() {
                {
                    let t = DropTimer::new("tmap");
                    if let Err(err) =
                        update_chunk_map(map.deref_mut(), &radar_scan, scan_pos, direction).await
                    {
                        println!("error in map {:?}", err);
                    }
                    drop(t);
                }
                Breakpoint::new().await;
                {
                    if let Err(err) = exploration.activate(scan_pos, &radar_scan) {
                        println!("error in exploration {:?}", err);
                    }
                }
                Breakpoint::new().await;
            }
        }
        Breakpoint::new().await;

        // update border of reachable terrain
        println!("ub");
        exploration.run(&mut map).await;
        if exploration.get_state().is_complete() && !exploration_completed {
            println!("map complete");
            exploration_completed = true;
        }
        Breakpoint::new().await;

        // reset destination if reached
        println!("rt");
        if destination == Some(scan_pos) {
            destination = None
        }
        // destination at border of known reachable
        if destination.is_none()
            && let Some(mut unknown_reachables) = exploration.border(&map)
        {
            fn update_closest(
                closest: &mut Option<(Position, u16)>,
                candidate: Option<(Position, u16)>,
            ) {
                if let Some(candidate) = candidate
                    && closest.is_none_or(|(_, dist_old)| candidate.1 < dist_old)
                {
                    *closest = Some(candidate);
                }
            }
            fn get_closest(
                iter: impl Iterator<Item = Position>,
                reference: Position,
            ) -> Option<(Position, u16)> {
                iter.map(|pos_border| {
                    (pos_border, DistanceBotWalk::measure(pos_border - reference))
                })
                .min_by_key(|&(_, dist)| dist)
            }

            let mut closest = None;
            loop {
                // split iterator into chunks to prevent to many await points
                match unknown_reachables.next_chunk::<5>() {
                    Ok(chunk) => {
                        update_closest(&mut closest, get_closest(chunk.into_iter(), scan_pos));
                    }
                    Err(iter) => {
                        update_closest(&mut closest, get_closest(iter, scan_pos));
                        break;
                    }
                }
                Breakpoint::new().await;
            }

            if let Some((destination_pos, _)) = closest {
                destination = Some(destination_pos);
                nav.initialize(scan_pos, destination_pos);
            }
        }
        Breakpoint::new().await;

        // react to movements that changed the starting position of navigation that changed the
        // starting position of navigation
        println!("nu");
        if let Some(task) = nav.get_state().task()
            && task.from != scan_pos
        {
            nav.update_start(scan_pos).unwrap();
        }
        Breakpoint::new().await;

        // navigation
        println!("nr");
        nav.run(|pos| map.get(pos).is_some_and(|t| t.is_known_walkable()))
            .await;

        Breakpoint::new().await;
        println!("ns");
        if nav.get_state().is_success() {
            println!("ns-");
            let mut destinations = Vec::<Position, 3>::new();
            let mut pos = scan_pos;
            let range = 2u16;
            let mut counter = 0u16;
            loop {
                let Some(&dir) = nav.next_step(pos).all().first() else {
                    break;
                };
                pos += Vec2::new_in_direction(dir, 1);
                counter = (counter + 1).rem_euclid(range);
                if counter.rem_euclid(range) == 0 {
                    let Ok(_) = destinations.push(pos) else { break };
                }
            }
            print!("ev {}", scan_pos);
            for pos in &destinations {
                print!("ev {}", pos);
            }
            println!();
            signal_nav.signal(NavigationEvaluationN::new(destinations, range));
        } else {
            // TODO there is an error
            println!("{:?}", nav.get_state());
            print_map(map.deref(), scan_pos, -3..=3, -3..=3, |pos| {
                if pos == scan_pos {
                    Some('@')
                } else if Some(pos) == destination {
                    Some('x')
                } else {
                    nav.get_dist_at(pos)
                        .map(|dist| dist.to_string().chars().last().unwrap())
                }
            });
        }
    }
}

fn print_map(
    map: &impl Map<Terrain>,
    pos: Position,
    range_east: RangeInclusive<i16>,
    range_south: RangeInclusive<i16>,
    markers: impl Fn(Position) -> Option<char>,
) {
    for south in range_south {
        for east in range_east.clone() {
            let pos_print = pos + Vec2::new_east_south(east, south);
            let ch = match markers(pos_print) {
                Some(ch) => ch,
                None => match map.get(pos_print) {
                    Some(Terrain::Reachable) => ' ',
                    Some(Terrain::Walkable) => '.',
                    Some(Terrain::Blocked) => '█',
                    _ => '░',
                },
            };
            print!("{}", ch);
        }
        println!("");
    }
}

type MyMap = ChunkMapHash<128, Terrain, ChunkTerrain>;
type MyNav = Navigation<ChunkMapHash<64, Option<NonZeroU16>, [[Option<NonZeroU16>; 8]; 8]>, 64>;
type MyExp = Exploration<256, MyMap>;

#[task]
async fn watchdog() -> ! {
    let mut dog = StatsDog::<KartoffelClock>::new();
    loop {
        dog.restart_timer();
        Breakpoint::new().await;
        dog.feed();

        if dog.total() > Duration::from_secs(15) {
            println!("{}", dog);
            dog = StatsDog::new();
        }
    }
}
