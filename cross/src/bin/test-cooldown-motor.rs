#![no_main]
#![no_std]
#![feature(custom_test_frameworks)]
#![test_runner(test_kartoffel::runner)]

use async_kartoffel::{Bot, Instant, println};
use embassy_executor::{Executor, task};
use static_cell::StaticCell;

#[unsafe(no_mangle)]
fn main() {
    static EXECUTOR: StaticCell<Executor> = StaticCell::new();
    let executor = EXECUTOR.init(Executor::new());

    executor.run(|spawner| {
        spawner.spawn(main_task(Bot::take())).unwrap();
    })
}

#[task]
async fn main_task(mut bot: Bot) -> ! {
    let mut time = Instant::now();
    loop {
        bot.motor.step_fw().await;
        let done = Instant::now();
        println!("Took: {:?}", (done - time).unwrap());
        time = done;
    }
}
