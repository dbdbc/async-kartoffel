#![no_main]
#![no_std]
#![feature(custom_test_frameworks)]
#![test_runner(test_kartoffel::runner)]
#![feature(iter_next_chunk)]

use async_kartoffel::{println, Bot, RadarSize, Rotation, Vec2, D7};
use embassy_executor::{task, Executor};
use example_kartoffels::{get_global_pos, global_pos_entries};
use kartoffel_gps::Chunk;
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
    let mut facing = bot.compass.direction().await;

    for chunk in global_pos_entries() {
        println!("{}", chunk);
    }

    loop {
        let scan = bot.radar.scan::<D7>().await;

        let mut chunk = Chunk::<7>::default();
        for i_east in D7::range() {
            for i_south in D7::range() {
                let vec = Vec2::new_east(i_east.into()) + Vec2::new_south(i_south.into());
                let vec_local = vec.local(facing);
                let walkable = scan.at(vec_local).unwrap().is_walkable_terrain();
                chunk.0[usize::try_from(i_south + 3).unwrap()]
                    [usize::try_from(i_east + 3).unwrap()] = walkable;
            }
        }
        let pos = get_global_pos(&chunk);
        if let Some((south, east)) = pos {
            println!("global pos (east, south): ({}, {})", east, south);
        } else {
            println!("global pos unknown");
        }

        if scan.at(Vec2::new_front(1)).unwrap().is_walkable_terrain() {
            bot.motor.step_fw().await;
        } else if scan.at(Vec2::new_right(1)).unwrap().is_walkable_terrain() {
            bot.motor.turn_right().await;
            facing += Rotation::Right;
        } else if scan.at(Vec2::new_left(1)).unwrap().is_walkable_terrain() {
            bot.motor.turn_left().await;
            facing += Rotation::Left;
        } else {
            bot.motor.turn_left().await;
            facing += Rotation::Left;
            bot.motor.turn_left().await;
            facing += Rotation::Left;
        }
    }
}
