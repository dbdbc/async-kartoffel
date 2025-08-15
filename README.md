# async-kartoffel

An abstraction layer for [kartoffels by Patryk27](https://codeberg.org/pwy/kartoffels) that is
intended to simplify the development of complex bots. It is mainly intended to be used with an async
runtime like [embassy](https://github.com/embassy-rs/embassy), but also (mostly) includes blocking variants.

## Try it out

In the `src/bin` directory there are a few example bots. These are:

| *name* | *description* |
|--------|---------------|
| challenge-roomba.rs | solution to the roomba challenge |
| runner-gps.rs | navigates to a hardcoded location, evading and killing bots on its way |
| runner-simple.rs | evades and kills bots |
| runner-slam.rs | evades and kills bots, while creating a map of the terrain |
| tutorial-stab.rs | stabbing tutorial with multiple concurrent tasks |
| tutorial-line.rs | line following tutorial with multiple concurrent tasks |

To build the bots, use
```bash
cargo build --release --bins
```
Then convert the binaries to base64 and copy to clipboard, e.g. like this (linux and wayland)
```bash
base64 target/riscv32-kartoffel-bot/release/runner-gps | wl-copy
```
For other systems, check the build script in the [default starter pack](https://github.com/Patryk27/kartoffel/).

## Crates

The different crates provide different functionality:

### `async-kartoffel`
- Contains safe and convenient abstractions `Motor`, `Arm`, `Radar`, ... For radar scan,
  an `Rc`-like implementation is used to prevent new scan from overwriting old ones.
- Easily keep track of absolute `Position`, relative position (`Vec2`) in global (north, east,
  south, west) and local (front, right, back, left) coordinate frames, `Rotation`s and `Direction`s.
- Moving the bot, stabbing, scanning etc. mutate global state. This is
  represented by encapsulating these functions in singletons. They can be aquired
  exactly once using `Bot::take()`. This way you can be sure that if you keep a
  reference to `motor`, and checked `motor.is_ready()`, the motor stays ready.
- `Instant` and `Duration` tyes for `Timer`s
- `async` API that does not block the execution of other code, like `motor.step_fw().await`. There
  are also non-async variants such as `motor.wait_blocking()` or `motor.try_step_fw()`.

### `async-algorithm`
- Contains mapping, exploration, and navigation utilities and algorithms.
- Algorithms are implemented in async functions, where special care was taken to ensure they don't
  block for too long, so that fast reaction times are still possible.
- `StatsDog`: Utility for gathering latency and execution time stats
- Measure distances: Manhattan (taxi-cab), minimum, maximum, bot clock cycles, ...

### `kartoffel-gps` and `kartoffel-gps-builder`
- Provided with a map of the terrain, the exact global location can be uniquely identified by
  analysing terrain features.
- The provided map is stored in memory and can be used.
- Additionally, the provided map is analysed at compile time to allow fast and efficient navigation
  to any location.

### `test-kartoffel`
- Can be used to write unit tests.

## Work in progress 🚧

There may be some bugs, especially in the `async-algorithm` crate. The `async-kartoffel` crate is
relatively stable.

Possible improvements:
- tests for binaries
- logging
- inventory
- timer queue, better wakers
- benchmarks and optimization

## Tips
### Analysing stack size
The bot only has 4096 bytes of stack memory. Large arrays can easily cause a stack overflow. To
prevent this, use the provided allocator to heap allocate these arrays. You can use
[cargo-call-stack](https://github.com/Dirbaio/cargo-call-stack)  (with updated ```llvm-sys``` to
match llvm version, at time of writing `llvm-sys = "201.0.1"`) to analyse the stack requirements of
each function.

```bash
cargo call-stack -i target/riscv32-kartoffel-bot/release/runner-gps --target riscv32-unknown-none -v > cg.dot
dot -Tsvg cg.dot > cg.svg
```

### Tests

The unstable `custom_test_frameworks` is used for test. The build command is e.g. (linux and wayland)
```bash
cargo build --release --tests --all
base64 target/riscv32-kartoffel-bot/release/deps/async_kartoffel-e58abfc84af62516 | wl-copy
```
The hash (`e58a...`) may need to be adapted.

Add the following lines 
```rust
#![feature(custom_test_frameworks)]
#![reexport_test_harness_main = "test_main"]
#![test_runner(test_kartoffel::runner)]
```
add the top of your library to enable tests. Might not work for binaries though.

### Running native binaries
```cargo run --release --package kartoffel-gps-builder --target x86_64-unknown-linux-gnu -Z build-std=core,std,alloc --bin analyze-map```

