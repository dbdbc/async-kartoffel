# async-kartoffel

An asynchronous abstraction layer for [kartoffels by
Patryk27](https://github.com/Patryk27/kartoffels/)

# How to use

See binaries in `src` directory. To build and copy to clipboard, use e.g. (linux and wayland)
```bash
cargo build --release --bins
base64 target/riscv32-kartoffel-bot/release/tutorial-stab | wl-copy`
```
For other systems, check the build script in the [default starter pack](https://github.com/Patryk27/kartoffel/).

# Work in progress 🚧

- the `async-algorithm` crate is possibly incorrect, slow, and may change at any moment

Possible improvements:
- tests for binaries
- logging
- inventory
- timer queue, better wakers
- benchmarks and optimization

# Features

- Concurrent execution is useful for managing multiple tasks (like navigation and immediate
  reactions to new information) at once. To use the async functions, an executor must be used, e.g.
  the `embassy_executor`. The API can also be used in a blocking paradigm.
- Moving the bot, stabbing, scanning etc. mutate global state. This is
  represented by encapsulating these functions in singletons. They can be aquired
  exactly once using `Bot::take()`. This way you can be sure that if you keep a
  reference to `motor`, and checked `motor.is_ready()`, the motor stays ready.
- If you tried to use `Motor`/`Arm`/... and they were not ready, an `Err` is returned.
- In the [default
  firmware](https://github.com/Patryk27/kartoffels/tree/main/app/crates/kartoffel),
  radar scan can be overwritten by new scans. In this API, this is no longer
  possible, which is enforced by preventing any new scans as long as not all
  radar scans have been dropped. It might be necessary to manually call
  `mem::drop` in certain scenarios.
- Bound checking radar scan
- `Instant` and `Duration` tyes for timers
- `Tile`, `Direction`, `Distance` types
- Unit Tests
- Interruptable and cooperative `Map` and `Navigation` in `async-algorithm`
  crate, to ensure low latency reactions to other bots.

# Tests

The unstable `custom_test_frameworks` is used for test. The build command is e.g. (linux and wayland)
```bash
cargo build --release --tests --all
base64 target/riscv32-kartoffel-bot/release/deps/async_kartoffel-e58abfc84af62516 | wl-copy
```
Hash (`e58a...`) may need to be adapted.

Add the following lines 
```rust
#![feature(custom_test_frameworks)]
#![reexport_test_harness_main = "test_main"]
#![test_runner(test_kartoffel::runner)]
```
add the top of your library to enable tests. Might not work for binaries though.

# Running native binaries

```cargo run --release --package kartoffel-gps-builder --target x86_64-unknown-linux-gnu -Z build-std=core,std,alloc --bin analyze-map```
