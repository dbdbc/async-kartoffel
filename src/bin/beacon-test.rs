#![no_main]
#![no_std]
#![feature(custom_test_frameworks)]
#![test_runner(test_kartoffel::runner)]
#![feature(iter_next_chunk)]

use core::convert::identity;

use async_kartoffel::{
    print, println, random_seed, Bot, Direction, Duration, Instant, RadarScan, RadarSize, Rotation,
    Vec2, D3, D7 as DRadar,
};
use embassy_executor::{task, Executor};
use example_kartoffels::{beacon_info, get_global_pos, get_navigator_info, make_navigator};
use kartoffel_gps::{
    beacon::Navigator,
    gps::{MapSection, MapSectionTrait},
    GlobalPos,
};
use rand::{rngs::SmallRng, seq::IndexedRandom, SeedableRng};
use static_cell::StaticCell;

#[no_mangle]
fn main() {
    static EXECUTOR: StaticCell<Executor> = StaticCell::new();
    let executor = EXECUTOR.init(Executor::new());

    executor.run(|spawner| {
        spawner.spawn(main_task(Bot::take())).unwrap();
    })
}

#[task]
async fn main_task(mut bot: Bot) -> ! {
    // TODO this can be implemented in async_kartoffel
    let mut rng = {
        let kartoffel_seed = random_seed().to_be_bytes();
        let mut seed = [0u8; 32];
        seed[0] = kartoffel_seed[0];
        seed[1] = kartoffel_seed[1];
        seed[2] = kartoffel_seed[2];
        seed[3] = kartoffel_seed[3];
        let rng = rand::rngs::SmallRng::from_seed(seed);
        rng
    };

    let mut facing = bot.compass.direction().await;
    let mut pos: Option<GlobalPos> = None;

    println!("creating navigator");
    let mut navigator = make_navigator();

    println!("setting target");
    let target = GlobalPos::add_to_anchor(Vec2::new_global(4, -5));

    println!("beacon test, trying to navigate to {}", target);
    println!("{:?}", beacon_info());

    println!("{:?}", get_navigator_info());

    loop {
        let scan = bot.radar.scan::<DRadar>().await;

        // update pos if new info is available
        match (pos, get_global_pos(&MapSection::from_scan(&scan, facing))) {
            (Some(old_pos), Some(new_pos)) => {
                if new_pos != old_pos {
                    println!("updated pos from {} to {}", old_pos, new_pos);
                    pos = Some(new_pos);
                }
            }
            (None, Some(new_pos)) => {
                println!("found position: {}", new_pos);
                pos = Some(new_pos);
            }
            (None, None) => println!("global pos unknown"),
            (Some(_), None) => {}
        }

        if let Some(pos) = pos.as_mut() {
            drop(scan);
            navigator.initialize(*pos, target);
            println!("starting computation");
            navigator
                .compute()
                .unwrap_or_else(|err| println!("computation failed: {:?}", err));

            if navigator.is_ready() {
                println!("start navigating");
                while !navigator.is_completed().unwrap() {
                    {
                        let scan = bot.radar.scan::<DRadar>().await;
                        if let Some(new_pos) = get_global_pos(&MapSection::from_scan(&scan, facing))
                        {
                            if new_pos != *pos {
                                println!("correction pos: {} -> {}", pos, new_pos);
                                *pos = new_pos;
                                if let Err(err) = navigator.move_start_to(*pos) {
                                    println!("moving start failed: {:?}", err);
                                }
                            }
                        }
                    }

                    let time_switch = Instant::now();
                    println!("nav {}", navigator.get_start().unwrap());
                    while (Instant::now() - time_switch).unwrap() < Duration::from_secs(5) {
                        let scan = bot.radar.scan::<D3>().await;
                        for dir in Direction::all() {
                            if navigator.is_dir_good(dir).is_some_and(identity)
                                && scan
                                    .at(Vec2::from_direction(dir, 1).local(facing))
                                    .is_some_and(|t| t.is_empty())
                            {
                                match dir - facing {
                                    Rotation::Id => {}
                                    Rotation::Left => {
                                        bot.motor.turn_left().await;
                                        facing += Rotation::Left;
                                    }
                                    Rotation::Right => {
                                        bot.motor.turn_right().await;
                                        facing += Rotation::Right;
                                    }
                                    Rotation::Inverse => {
                                        for _ in 0..2 {
                                            bot.motor.turn_right().await;
                                            facing += Rotation::Right;
                                        }
                                    }
                                }
                                bot.motor.step_fw().await;
                                print!(".");
                                *pos += Vec2::new_front(1).global(facing);
                                if let Err(err) = navigator.move_start_to(*pos) {
                                    println!("moving start failed: {:?}", err);
                                }
                                break;
                            }
                        }
                    }
                    println!("random {}", navigator.get_start().unwrap());
                    while (Instant::now() - time_switch).unwrap() < Duration::from_secs(10) {
                        let scan = bot.radar.scan::<D3>().await;
                        let dir = random_direction(scan, &mut rng, facing);
                        if let Some(dir) = dir {
                            match dir - facing {
                                Rotation::Id => {}
                                Rotation::Left => {
                                    bot.motor.turn_left().await;
                                    facing += Rotation::Left;
                                }
                                Rotation::Right => {
                                    bot.motor.turn_right().await;
                                    facing += Rotation::Right;
                                }
                                Rotation::Inverse => {
                                    bot.motor.turn_right().await;
                                    bot.motor.turn_right().await;
                                    facing += Rotation::Inverse;
                                }
                            }
                            bot.motor.step_fw().await;
                            print!(".");
                            *pos += Vec2::new_front(1).global(facing);
                            if let Err(err) = navigator.move_start_to(*pos) {
                                println!("moving start failed: {:?}", err);
                            }
                        } else {
                            println!("we're stuck :(");
                        }
                    }
                }
            } else {
                println!("no path found");
            }

            println!("done navigating");

            loop {}
            // TODO
        } else {
            // do random step until pos is known
            let dir = random_direction(scan, &mut rng, facing);
            if let Some(dir) = dir {
                match dir - facing {
                    Rotation::Id => {}
                    Rotation::Left => {
                        bot.motor.turn_left().await;
                        facing += Rotation::Left;
                    }
                    Rotation::Right => {
                        bot.motor.turn_right().await;
                        facing += Rotation::Right;
                    }
                    Rotation::Inverse => {
                        bot.motor.turn_right().await;
                        bot.motor.turn_right().await;
                        facing += Rotation::Inverse;
                    }
                }
                bot.motor.step_fw().await;
                print!(".");
            } else {
                println!("we're stuck :(");
            }
        }
    }
}

fn random_direction<D: RadarSize>(
    scan: RadarScan<D>,
    rng: &mut SmallRng,
    facing: Direction,
) -> Option<Direction> {
    let available_directions = Direction::all()
        .into_iter()
        .filter(|&dir| {
            scan.at(Vec2::from_rotation(dir - facing, 1))
                .unwrap()
                .is_empty()
        })
        .collect::<heapless::Vec<Direction, 4>>();
    available_directions.choose(rng).map(|dir| *dir)
}
