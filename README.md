# async-kartoffel
An asynchronous alternative firmware for [kartoffels by
Patryk27](https://github.com/Patryk27/kartoffels/)

# Work in progress 🚧
Missing:
- usage example that includes build system
- tests
- logging?
- inventory?
- timer queue, better wakers?

# Features
- Concurrent execution is useful for managing multiple tasks (like navigation and immediate
  reactions to new information) at once. To use the async functions, an executor must be used, e.g.
  the embassy_executor.
- Movement the bot, stabbing, scanning etc. mutate global state, this is represented by
  encapsulating these functions in singletons that can be accessed using `Bot::take()`.
- Radar scan data is now longer accessible after a new scan, this is represented by preventing any
  new scans as long as not all radar scans have been dropped.
- Bound checking radar scan
- Instant and Duration tyes for timers
- `Tile`, `Direction`, `Distance` types
- `print` and `println` macros

# Minimal usage example

```rust
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
```
