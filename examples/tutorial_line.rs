#![no_main]
#![no_std]

use async_kartoffel::{println, Bot, Distance, D3};
use embassy_executor::{task, Executor};
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
    loop {
        let scan = bot.radar.scan::<D3>().await;
        if scan.at(Distance::new_front(1)).unwrap().is_empty() {
            bot.motor.step().await;
        } else if scan.at(Distance::new_right(1)).unwrap().is_empty() {
            bot.motor.turn_right().await;
        } else if scan.at(Distance::new_left(1)).unwrap().is_empty() {
            bot.motor.turn_left().await;
        } else {
            println!("reached end of line")
        }
    }
}
