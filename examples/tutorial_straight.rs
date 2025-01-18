#![no_main]
#![no_std]

use async_kartoffel::{println, Bot, Duration, Timer};
use embassy_executor::{task, Executor};
use static_cell::StaticCell;

#[no_mangle]
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
        bot.motor.step().await;
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
