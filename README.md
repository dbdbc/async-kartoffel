# async-kartoffel
An asynchronous abstraction layer for [kartoffels by
Patryk27](https://github.com/Patryk27/kartoffels/)

# Work in progress 🚧
Missing:
- usage example that includes build system
- tests
- logging?
- inventory?
- timer queue, better wakers?
- benchmarks and optimization

# Features
- Concurrent execution is useful for managing multiple tasks (like navigation and immediate
  reactions to new information) at once. To use the async functions, an executor must be used, e.g.
  the embassy_executor.
- Moving the bot, stabbing, scanning etc. mutate global state, this is represented by
  encapsulating these functions in singletons that can be accessed using `Bot::take()`.
- Radar scan data is no longer accessible after a new scan, this is achieved by preventing any
  new scans as long as not all radar scans have been dropped.
- Bound checking radar scan
- `Instant` and `Duration` tyes for timers
- `Tile`, `Direction`, `Distance` types
- `print` and `println` macros
- Tests

# Examples

See examples directory, build command e.g. (linux and wayland)
`cargo build --release --examples && base64 target/riscv64-kartoffel-bot/release/examples/tutorial_straight | wl-copy`

# Tests

Using unstable `custom_test_frameworks`, build command e.g. (linux and wayland)
`cargo build --release --tests && base64 target/riscv64-kartoffel-bot/release/deps/async_kartoffel-e58abfc84af62516 | wl-copy`
Hash may need to be adapted.
