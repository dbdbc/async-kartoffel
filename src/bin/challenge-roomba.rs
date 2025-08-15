#![no_main]
#![no_std]
#![feature(custom_test_frameworks)]
#![test_runner(test_kartoffel::runner)]
#![feature(iter_next_chunk)]

use alloc::boxed::Box;
use async_algorithm::{
    Breakpoint, ChunkMapHash, ChunkTerrain, DistanceBotWalk, DistanceManhattan, DistanceMeasure,
    Exploration, Map, Navigation, StatsDog, Terrain, distance_walk_with_rotation, update_chunk_map,
};
use async_kartoffel::{
    Bot, D5, Direction, Instant, Local, Motor, Position, Radar, RadarScan, RadarScanWeak,
    RadarSize, Rotation, Tile, Vec2, println,
};
use core::num::NonZeroU16;
use core::ops::DerefMut;
use embassy_executor::{Executor, task};
use embassy_futures::select::{Either, select};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, signal::Signal};
use heapless::Vec;
use static_cell::StaticCell;

extern crate alloc;

#[unsafe(no_mangle)]
fn main() {
    static EXECUTOR: StaticCell<Executor> = StaticCell::new();
    static SIGNAL_MAP: StaticCell<Signal<NoopRawMutex, MapUpdate>> = StaticCell::new();
    static SIGNAL_NAVIGATION: StaticCell<Signal<NoopRawMutex, Position>> = StaticCell::new();
    static SIGNAL_COMPLETE: StaticCell<Signal<NoopRawMutex, ()>> = StaticCell::new();

    let executor = EXECUTOR.init(Executor::new());
    let signal_map = SIGNAL_MAP.init(Signal::new());
    let signal_navigation = SIGNAL_NAVIGATION.init(Signal::new());
    let signal_complete = SIGNAL_COMPLETE.init(Signal::new());

    let map: Box<MyMap> = Default::default();
    let nav: Box<MyNav> = Default::default();
    let exploration: Box<MyExp> = Default::default();

    println!("async_kartoffel");

    executor.run(|spawner| {
        spawner
            .spawn(foreground(Bot::take(), signal_map, signal_navigation))
            .unwrap();
        spawner
            .spawn(background(
                map,
                nav,
                exploration,
                signal_map,
                signal_navigation,
                signal_complete,
            ))
            .unwrap();
        spawner.spawn(watchdog(signal_complete)).unwrap();
    })
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
    fn offset(&self) -> Vec2<Local> {
        match self {
            MotorAction::Step => Vec2::new_front(1),
            _ => Vec2::default(),
        }
    }
    fn rotation(&self) -> Rotation {
        match self {
            MotorAction::Step => Rotation::Id,
            MotorAction::TurnRight => Rotation::Right,
            MotorAction::TurnLeft => Rotation::Left,
        }
    }
}

struct MapUpdate {
    scan: RadarScanWeak<D5>,
    scan_pos: Position,
    direction: Direction,
}

async fn execute_until_radar_ready(
    radar: &Radar,
    motor: &mut Motor,
    motor_action: MotorAction,
    position: &mut Position,
    direction: &mut Direction,
) -> bool {
    match select(radar.wait(), motor_action.execute(motor)).await {
        Either::First(_) => false,
        Either::Second(_) => {
            *direction += motor_action.rotation();
            *position += motor_action.offset().global(*direction);
            true
        }
    }
}

fn movement<Size: RadarSize>(
    pos: Position,
    direction: Direction,
    radar_scan: &RadarScan<Size>,
    scan_pos: Position,
    navigation_destination: Option<Position>,
) -> Option<MotorAction> {
    MotorAction::ALL_AND_NOTHING
        .into_iter()
        .filter_map(|motor_action| {
            let translation = motor_action.map_or(Default::default(), |ma| ma.offset());
            let rotation = motor_action.map_or(Default::default(), |ma| ma.rotation());
            if translation == Vec2::default()
                || radar_scan
                    .at((pos - scan_pos).local(direction) + translation)
                    .is_some_and(|tile| tile.is_empty())
            {
                let eval = navigation_destination.map_or(0, |destination| {
                    let pos_next = pos + translation.global(direction);
                    let ori_next = direction + rotation;
                    distance_walk_with_rotation(destination - pos_next, ori_next)
                });
                Some((motor_action, eval))
            } else {
                None
            }
        })
        .min_by_key(|&(motor_action, eval)| {
            // this is the values that are minimized, in that order
            (
                eval,
                motor_action.is_some(),                  // prefer no movement
                motor_action == Some(MotorAction::Step), // prefer not moving forward
            )
        })
        .map(|(movement, _eval)| movement)?
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

    let mut nav_destination: Option<Position> = None;

    'main_loop: loop {
        let radar_scan = &radar.scan::<D5>().await;
        let scan_pos = pos;
        signal_map.signal(MapUpdate {
            scan: radar_scan.weak(),
            scan_pos,
            direction,
        });

        if radar_scan.at(Vec2::new_front(1)) == Some(Tile::Flag) {
            arm.pick().await;
        }

        // update navigation destination, if background task has provided a new update
        if let Some(destination) = signal_nav.try_take() {
            nav_destination = Some(destination);
        }

        while let Some(motor_action) =
            movement(pos, direction, radar_scan, scan_pos, nav_destination)
        {
            if !execute_until_radar_ready(radar, motor, motor_action, &mut pos, &mut direction)
                .await
            {
                continue 'main_loop;
            }
            Breakpoint::new().await;
        }
    }
}

type MyMap = ChunkMapHash<128, Terrain, ChunkTerrain>;
type MyNav = Navigation<ChunkMapHash<64, Option<NonZeroU16>, [[Option<NonZeroU16>; 8]; 8]>, 64>;
type MyExp = Exploration<256, MyMap>;

#[allow(unused)]
struct DropTimer<'a> {
    init: Instant,
    name: &'a str,
}
impl<'a> DropTimer<'a> {
    #[allow(unused)]
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

#[task]
async fn background(
    mut map: Box<MyMap>,
    mut nav: Box<MyNav>,
    mut exploration: Box<MyExp>,
    signal_map: &'static Signal<NoopRawMutex, MapUpdate>,
    signal_nav: &'static Signal<NoopRawMutex, Position>,
    signal_complete: &'static Signal<NoopRawMutex, ()>,
) -> ! {
    map.set(Default::default(), Terrain::Walkable).unwrap();
    exploration.initialize(&mut map, Default::default());

    let mut destination: Option<Position> = None;
    let mut exploration_completed = false;
    let mut flags = Vec::<Position, 4>::new();
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
                    // let _t = DropTimer::new("tmap");
                    if let Err(err) =
                        update_chunk_map(map.deref_mut(), &radar_scan, scan_pos, direction).await
                    {
                        println!("error in map {:?}", err);
                    }
                }
                Breakpoint::new().await;
                {
                    // let _t = DropTimer::new("texp");
                    if let Err(err) = exploration.activate(scan_pos, &radar_scan) {
                        println!("error in exploration {:?}", err);
                    }
                }
                Breakpoint::new().await;

                // keep only flags that are not updated by this scan
                flags = flags
                    .into_iter()
                    .filter(|&flag_pos| {
                        !radar_scan.contains((flag_pos - scan_pos).local(direction))
                    })
                    .collect();
                Breakpoint::new().await;
                for vec in radar_scan.iter_tile(Tile::Flag) {
                    let flag_pos = scan_pos + vec.global(direction);
                    flags.push(flag_pos).expect("more than 4 flags found");
                }
            }
        }
        Breakpoint::new().await;

        // update border of reachable terrain
        exploration.run(&mut map).await;
        if exploration.get_state().is_complete() && !exploration_completed {
            println!("map complete");
            exploration_completed = true;
            signal_complete.signal(());
        }
        Breakpoint::new().await;

        // reset destination if reached
        if destination == Some(scan_pos) {
            destination = None
        }
        // flags are priority destination
        if destination.is_none_or(|destination| !flags.contains(&destination))
            && let Some(&destination_flag) = flags
                .iter()
                .filter(|&&flag_pos| {
                    map.get(flag_pos)
                        .is_some_and(|terrain| terrain == Terrain::Reachable)
                })
                .min_by_key(|&&flag_pos| DistanceManhattan::measure(flag_pos - scan_pos))
        {
            destination = Some(destination_flag);
            nav.initialize(scan_pos, destination_flag);
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
        if let Some(task) = nav.get_state().task()
            && task.from != scan_pos
        {
            nav.update_start(scan_pos).unwrap();
        }
        Breakpoint::new().await;

        // navigation
        nav.run(|pos| map.get(pos).is_some_and(|t| t.is_known_walkable()))
            .await;

        Breakpoint::new().await;
        if nav.get_state().is_success() {
            signal_nav.signal(
                scan_pos
                    + nav
                        .next_step(scan_pos)
                        .all()
                        .first()
                        .map_or(Vec2::default(), |&dir| Vec2::new_in_direction(dir, 1)),
            );
        }
    }
}

#[task]
async fn watchdog(signal_complete: &'static Signal<NoopRawMutex, ()>) -> ! {
    let mut dog = StatsDog::new();
    loop {
        dog.restart_timer();
        Breakpoint::new().await;
        dog.feed();

        if signal_complete.try_take().is_some() {
            println!("{}", dog);
        }
    }
}
