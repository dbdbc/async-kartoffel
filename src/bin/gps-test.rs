#![no_main]
#![no_std]
#![feature(custom_test_frameworks)]
#![test_runner(test_kartoffel::runner)]
#![feature(iter_next_chunk)]

use async_kartoffel::{println, Bot, Instant, Rotation, Vec2, D7 as DRadar};
use embassy_executor::{task, Executor};
use example_kartoffels::{get_global_pos, global_pos_entries};
use kartoffel_gps::gps::{Chunk, MapSection};
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

    println!("n unique: {:?}", global_pos_entries().count());

    loop {
        let scan = bot.radar.scan::<DRadar>().await;

        let chunk = Chunk::from_scan(&scan, facing);
        let t = Instant::now();
        let pos = get_global_pos(&chunk);
        let dur = (Instant::now() - t).unwrap();
        if let Some(gpos) = pos {
            println!("global pos: ({})", gpos);
        } else {
            println!("global pos unknown");
        }
        println!("{}", dur);

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
