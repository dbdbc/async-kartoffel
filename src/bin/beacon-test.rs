#![no_main]
#![no_std]
#![feature(custom_test_frameworks)]
#![test_runner(test_kartoffel::runner)]
#![feature(iter_next_chunk)]

use async_kartoffel::{
    println, random_seed, Bot, Direction, Instant, Rotation, Vec2, D7 as DRadar,
};
use embassy_executor::{task, Executor};
use example_kartoffels::{
    beacon_graph, beacon_info, beacons, get_global_pos, global_pos_entries, map,
};
use kartoffel_gps::{
    beacon,
    gps::{MapSection, MapSectionTrait},
    GlobalPos,
};
use rand::{distr, seq::IndexedRandom, Rng, SeedableRng};
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
    let mut pos = None;

    // let mut navigator = beacon::Nav::new(map(), beacon_graph(), beacons(), beacon_info());

    let target = GlobalPos::add_to_anchor(Vec2::new_global(46, -64));

    println!("beacon test, trying to navigate to {}", target);
    println!("{:?}", beacon_info());

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

        if let Some(pos) = pos {
            // TODO
        } else {
            // do random step
            let available_directions = Direction::all()
                .into_iter()
                .filter(|&dir| {
                    scan.at(Vec2::from_rotation(dir - facing, 1))
                        .unwrap()
                        .is_walkable_terrain()
                })
                .collect::<heapless::Vec<Direction, 4>>();
            let dir = available_directions.choose(&mut rng);
            if let Some(&dir) = dir {
                match dir - facing {
                    Rotation::Id => bot.motor.step_fw().await,
                    Rotation::Left => {
                        bot.motor.turn_left().await;
                        facing += Rotation::Left;
                        bot.motor.step_fw().await;
                    }
                    Rotation::Right => {
                        bot.motor.turn_right().await;
                        facing += Rotation::Right;
                        bot.motor.step_fw().await;
                    }
                    Rotation::Inverse => {
                        bot.motor.turn_right().await;
                        bot.motor.turn_right().await;
                        facing += Rotation::Inverse;
                        bot.motor.step_fw().await;
                    }
                }
            } else {
                println!("we're stuck :(");
            }
        }
    }
}
