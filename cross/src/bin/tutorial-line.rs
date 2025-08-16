#![no_main]
#![no_std]
#![feature(custom_test_frameworks)]
#![test_runner(test_kartoffel::runner)]
#![feature(iter_next_chunk)]

use async_kartoffel::{Bot, Duration, Timer, println};
use async_kartoffel_generic::{D3, RadarScanTrait, Tile, Vec2};
use embassy_executor::{Executor, task};
use static_cell::StaticCell;

#[unsafe(no_mangle)]
fn main() {
    static EXECUTOR: StaticCell<Executor> = StaticCell::new();
    let executor = EXECUTOR.init(Executor::new());

    executor.run(|spawner| {
        spawner.spawn(main_task(Bot::take())).unwrap();
        spawner.spawn(print_task()).unwrap();
    })
}

#[task]
async fn main_task(mut bot: Bot) -> ! {
    loop {
        let scan = bot.radar.scan::<D3>().await;
        if scan.at(Vec2::new_front(1)) == Some(Tile::Empty) {
            bot.motor.step_fw().await;
        } else if scan.at(Vec2::new_right(1)) == Some(Tile::Empty) {
            bot.motor.turn_right().await;
        } else if scan.at(Vec2::new_left(1)) == Some(Tile::Empty) {
            bot.motor.turn_left().await;
        }
    }
}

#[task]
async fn print_task() -> ! {
    let mut counter = 0;
    loop {
        Timer::after(Duration::from_secs(1)).await;
        counter += 1;
        println!("{}", counter);
    }
}
