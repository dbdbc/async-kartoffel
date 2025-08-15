#![no_main]
#![no_std]

use async_kartoffel::{println, Bot, Duration, Timer, Vec2, D3};
use embassy_executor::{task, Executor};
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
        if scan.at(Vec2::new_front(1)).unwrap().is_bot() {
            bot.arm.stab().await;
        } else if scan.at(Vec2::new_right(1)).unwrap().is_bot() {
            bot.motor.turn_right().await;
            bot.arm.stab().await;
        } else if scan.at(Vec2::new_left(1)).unwrap().is_bot() {
            bot.motor.turn_left().await;
            bot.arm.stab().await;
        } else if scan.at(Vec2::new_back(1)).unwrap().is_bot() {
            bot.motor.turn_left().await;
            bot.motor.turn_left().await;
            bot.arm.stab().await;
        } else {
            bot.motor.step_fw().await;
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
