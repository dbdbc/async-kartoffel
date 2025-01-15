# async-kartoffel
An asynchronous alternative firmware for kartoffels by Patryk27

# Basic ideas
- concurrent execution is useful managing multiple tasks (like navigation and immediate reactions to
  new information) at once
- moving the bot, stabbing, scanning etc. mutate global state, this is represented by encapsulating
  these functions in singletons (`Motor`, `Arm`, `Radar`, `Compass`)
- radar scan data is now longer accessible after a new scan happened, this is represented by
  preventing new scan as long as not all radar scans have been dropped
- it is nice to give the user feedback about whether certain actions succeeded or failed due to
  cooldown
- Instant and Duration are convenient types for timers
