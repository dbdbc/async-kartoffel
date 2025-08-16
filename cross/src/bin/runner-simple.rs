#![no_main]
#![no_std]
#![feature(custom_test_frameworks)]
#![test_runner(test_kartoffel::runner)]
#![feature(iter_next_chunk)]

use async_kartoffel::{Arm, Bot, Instant, Motor, Radar, RadarScan, Timer, println};
use async_kartoffel_generic::{
    D7, Direction, Duration, Local, Position, RadarScanTrait, RadarSize, Rotation, Tile, Transform,
    Vec2,
};
use embassy_executor::{Executor, task};
use embassy_futures::select::{Either, Either3, select, select3};
use heapless::Vec;
use static_cell::StaticCell;

extern crate alloc;

#[unsafe(no_mangle)]
fn main() {
    static EXECUTOR: StaticCell<Executor> = StaticCell::new();

    let executor = EXECUTOR.init(Executor::new());

    println!("async_kartoffel");
    println!("7 synchronized");
    println!("bot score 3 back bots len 0x4");
    println!("terrain back against wall");

    executor.run(|spawner| {
        spawner.spawn(foreground(Bot::take())).unwrap();
    })
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
    fn to_transform(motor: Option<MotorAction>) -> Transform {
        match motor {
            Some(MotorAction::Step) => Transform::from(Vec2::new_front(1)),
            Some(MotorAction::TurnLeft) => Transform::from(Rotation::Left),
            Some(MotorAction::TurnRight) => Transform::from(Rotation::Right),
            Some(MotorAction::StepBack) => Transform::from(Vec2::new_back(1)),
            None => Transform::identity(),
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

async fn execute_with_arm_timeout(
    radar: &mut Radar,
    motor: &mut Motor,
    arm: &mut Arm,
    action: &MotorArmAction,
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
                MotorAction::StepBack => *position += Vec2::new_back(1).global(*direction),
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
        [0, 1, 1, 5, 1, 1, 0],
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

fn movement(
    _pos: Position,
    _direction: Direction,
    after_scan: Transform,
    radar_scan: &RadarScan<impl RadarSize>,
    bots: &[Vec2<Local>],
    can_stab: bool,
    time_stamp: Instant,
) -> MotorArmAction {
    let max_stab_wait = Duration::from_ticks(10_000);
    let (motor, stab) = MotorAction::ALL_AND_NOTHING
        .into_iter()
        .filter_map(|movement| {
            let t = after_scan.chain(MotorAction::to_transform(movement));
            let next_location = t.translation();
            let possible = radar_scan
                .at(next_location)
                .is_some_and(|tile| tile.is_walkable_terrain());
            let back_against_wall = radar_scan
                .at(t.chain(Vec2::new_back(1).into()).translation())
                .is_some_and(|tile| !tile.is_walkable_terrain());
            if possible {
                // add evaluation for all bots
                let (bot_eval, stab) = bots
                    .iter()
                    .map(|bot| {
                        bot_eval_func(
                            t.inverse().chain((*bot).into()).translation(),
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
                        .at(t.chain(vec.into()).translation())
                        .map(|tile| tile.is_walkable_terrain())
                });

                // long-term evaluation
                Some((movement, stab, bot_eval, wall_eval))
            } else {
                None
            }
        })
        .min_by_key(|&(movement, _stab, bot_eval, wall_eval)| {
            // this is the values that are minimized, in that order
            // TODO which order is best?
            (
                // disincentivise backward with bot score due to cooldown cost
                bot_eval
                    + if movement == Some(MotorAction::StepBack) && !bots.is_empty() {
                        3
                    } else {
                        0
                    },
                wall_eval,
                // prefer forward
                movement != Some(MotorAction::Step),
                // prefer no movement if there are bots, prefer movement if there are no bots
                // movement.is_some() != (bots.len() == 0),
                movement.is_some(),
            )
        })
        .map(|(movement, stab, _, _)| (movement, stab))
        .unwrap_or((None, false));
    MotorArmAction {
        motor,
        arm: if stab { Some(ArmAction::Stab) } else { None },
        arm_timeout: time_stamp + max_stab_wait,
    }
}

/// get a list of all bots (that have not yet been stabbed) in the scan area
fn get_bot_list<const MAX_N_BOTS: usize, D: RadarSize>(
    transform: Transform,
    has_stabbed: bool,
    radar_scan: &RadarScan<D>,
) -> Vec<Vec2<Local>, MAX_N_BOTS> {
    let stabbed_location =
        has_stabbed.then_some(transform.chain(Vec2::new_front(1).into()).translation());
    if let Some(stabbed_location) = stabbed_location {
        radar_scan
            .iter_tile(Tile::Bot)
            .filter(|&v| v != stabbed_location)
            .collect()
    } else {
        radar_scan.iter_tile(Tile::Bot).collect()
    }
}

#[task]
async fn foreground(mut bot: Bot) -> ! {
    // settings
    const MAX_N_BOTS: usize = 28;

    let radar = &mut bot.radar;
    let arm = &mut bot.arm;
    let motor = &mut bot.motor;

    let mut pos = Position::default();

    let mut direction = bot.compass.try_direction().unwrap();

    'main_loop: loop {
        let radar_scan = &radar.scan::<D7>().await;
        let radar_timestamp = Instant::now();

        let action = instincts(arm, motor, radar_scan, radar_timestamp);
        if !execute_with_arm_timeout(radar, motor, arm, &action, &mut pos, &mut direction).await {
            continue 'main_loop;
        } else if action.motor.is_some() {
            // keep radar and motor in sync
            continue 'main_loop;
        } else {
            let after_scan = MotorAction::to_transform(action.motor);
            let bots = get_bot_list::<MAX_N_BOTS, _>(after_scan, action.arm.is_some(), radar_scan);
            let can_stab = arm.is_ready();
            let action = movement(
                pos,
                direction,
                after_scan,
                radar_scan,
                &bots,
                can_stab,
                radar_timestamp,
            );
            if !execute_with_arm_timeout(radar, motor, arm, &action, &mut pos, &mut direction).await
            {
                continue 'main_loop;
            }
        }
    }
}
