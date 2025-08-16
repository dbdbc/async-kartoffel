#![no_main]
#![no_std]
#![feature(custom_test_frameworks)]
#![test_runner(test_kartoffel::runner)]
#![feature(iter_next_chunk)]

use async_kartoffel::{Bot, Duration, Instant, RadarScan, Timer, println, random_seed};
use async_kartoffel_generic::{
    D3, D7 as DRadar, Direction, RadarScanTrait, RadarSize, Rotation, Vec2,
};
use embassy_executor::{Executor, task};
use example_kartoffels::{beacon_info, get_global_pos, get_navigator_info, navigator_resources};
use kartoffel_gps::{
    GlobalPos,
    beacon::Navigator,
    gps::{MapSection, MapSectionTrait},
    pos::pos_east_south,
};
use rand::{SeedableRng, rngs::SmallRng, seq::IndexedRandom};
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
    // TODO this can be implemented in async_kartoffel
    let mut rng = {
        let kartoffel_seed = random_seed().to_be_bytes();
        let mut seed = [0u8; 32];
        seed[0] = kartoffel_seed[0];
        seed[1] = kartoffel_seed[1];
        seed[2] = kartoffel_seed[2];
        seed[3] = kartoffel_seed[3];
        rand::rngs::SmallRng::from_seed(seed)
    };

    let mut facing = bot.compass.direction().await;

    println!("creating navigator resources");
    let resources = navigator_resources();

    println!("creating navigator");
    let navigator = Navigator::new(resources);

    let destinations = &[
        pos_east_south(4, 5),
        pos_east_south(62, 53),
        pos_east_south(125, 9),
        pos_east_south(8, 62),
    ];

    println!("setting target");
    let navigator = navigator.set_destination(*destinations.first().unwrap());

    println!(
        "beacon test, trying to navigate to {}",
        navigator.get_destination()
    );
    println!("{:?}", beacon_info());
    println!("{:?}", get_navigator_info());

    println!("random walk");
    let mut pos = random_walk(&mut bot, &mut facing, &mut rng).await;
    println!("found position: {}", pos);
    let mut navigator_outer = navigator.set_start(pos);

    let mut loop_count: usize = 0;
    loop {
        println!("starting computation");
        let mut navigator = navigator_outer
            .compute()
            .await
            .unwrap_or_else(|nav| panic!("computation failed: {:?}", nav.get_error()));

        println!("start navigating");
        while !navigator.is_completed() {
            {
                // position update if out of sync
                let scan = bot.radar.scan::<DRadar>().await;
                if let Some(new_pos) = get_global_pos(&MapSection::from_scan(&scan, facing))
                    && new_pos != pos
                {
                    println!("correction pos: {} -> {}", pos, new_pos);
                    pos = new_pos;
                    navigator = match navigator.set_start(pos).compute().await {
                        Ok(nav) => nav,
                        Err(nav) => {
                            // panic because position is known exactly, so this means
                            // navigation is really impossible
                            panic!("update comp failed: {:?}", nav.get_error())
                        }
                    };
                }
            }

            let time_switch = Instant::now();

            println!("random {}", navigator.get_start());
            let mut navigator_idle = {
                let start = navigator.get_start();
                navigator.set_start(start)
            };
            while (Instant::now() - time_switch).unwrap() < Duration::from_secs(10) {
                let scan = bot.radar.scan::<D3>().await;
                let dir = random_direction(scan, &mut rng, facing);
                if let Some(dir) = dir {
                    match dir - facing {
                        Rotation::Id => {}
                        Rotation::Left => {
                            bot.motor.turn_left().await;
                            facing += Rotation::Left;
                        }
                        Rotation::Right => {
                            bot.motor.turn_right().await;
                            facing += Rotation::Right;
                        }
                        Rotation::Inverse => {
                            bot.motor.turn_right().await;
                            bot.motor.turn_right().await;
                            facing += Rotation::Inverse;
                        }
                    }
                    bot.motor.step_fw().await;
                    pos += Vec2::new_front(1).global(facing);
                    navigator_idle = navigator_idle.set_start(pos);
                } else {
                    println!("we're stuck :(");
                }
            }
            navigator = match navigator_idle.set_start(pos).compute().await {
                Ok(nav) => nav,
                Err(nav) => {
                    println!("update comp failed: {:?}", nav.get_error());
                    println!("random walk");
                    pos = random_walk(&mut bot, &mut facing, &mut rng).await;
                    println!("found position: {}", pos);
                    let nav = nav.set_start(pos);
                    nav.compute().await.map_err(|nav| nav.get_error()).unwrap()
                }
            };
            {
                let beacons = navigator.get_beacons();
                let length = beacons.len();
                let tail = &beacons[length.saturating_sub(5)..];
                println!("{}: {:?}", length, tail);
            }

            println!("nav {}", navigator.get_start());
            while (Instant::now() - time_switch).unwrap() < Duration::from_secs(30)
                && !navigator.is_completed()
            {
                let scan = bot.radar.scan::<D3>().await;
                for dir in Direction::all() {
                    if (navigator.next_trivial_target() - navigator.get_start())
                        .directions()
                        .contains(&dir)
                        && scan
                            .at(Vec2::new_in_direction(dir, 1).local(facing))
                            .is_some_and(|t| t.is_empty())
                    {
                        match dir - facing {
                            Rotation::Id => {}
                            Rotation::Left => {
                                bot.motor.turn_left().await;
                                facing += Rotation::Left;
                            }
                            Rotation::Right => {
                                bot.motor.turn_right().await;
                                facing += Rotation::Right;
                            }
                            Rotation::Inverse => {
                                for _ in 0..2 {
                                    bot.motor.turn_right().await;
                                    facing += Rotation::Right;
                                }
                            }
                        }
                        bot.motor.step_fw().await;
                        // print!("{} ", navigator.get_beacons().len());
                        {
                            let beacons = navigator.get_beacons();
                            let length = beacons.len();
                            let tail = &beacons[length.saturating_sub(5)..];
                            println!("{}: {:?}", length, tail);
                        }
                        pos += Vec2::new_front(1).global(facing);
                        navigator = match navigator.set_start(pos).compute().await {
                            Ok(nav) => nav,
                            Err(nav) => {
                                println!("update comp failed: {:?}", nav.get_error());
                                println!("random walk");
                                pos = random_walk(&mut bot, &mut facing, &mut rng).await;
                                println!("found position: {}", pos);
                                let nav = nav.set_start(pos);
                                nav.compute().await.map_err(|nav| nav.get_error()).unwrap()
                            }
                        };
                        break;
                    }
                }
            }
        }

        println!("done navigating");
        Timer::after_secs(2).await;

        loop_count = (loop_count + 1).rem_euclid(destinations.len());

        navigator_outer = navigator.set_destination(destinations[loop_count]);
        println!("new destination: {}", navigator_outer.get_destination());
    }
}

// do random step until position
async fn random_walk(bot: &mut Bot, facing: &mut Direction, rng: &mut SmallRng) -> GlobalPos {
    loop {
        let scan = bot.radar.scan::<DRadar>().await;

        if let Some(pos) = get_global_pos(&MapSection::from_scan(&scan, *facing)) {
            return pos;
        }

        let dir = random_direction(scan, rng, *facing);
        if let Some(dir) = dir {
            match dir - *facing {
                Rotation::Id => {}
                Rotation::Left => {
                    bot.motor.turn_left().await;
                    *facing += Rotation::Left;
                }
                Rotation::Right => {
                    bot.motor.turn_right().await;
                    *facing += Rotation::Right;
                }
                Rotation::Inverse => {
                    bot.motor.turn_right().await;
                    bot.motor.turn_right().await;
                    *facing += Rotation::Inverse;
                }
            }
            bot.motor.step_fw().await;
        }
    }
}

fn random_direction<D: RadarSize>(
    scan: RadarScan<D>,
    rng: &mut SmallRng,
    facing: Direction,
) -> Option<Direction> {
    let available_directions = Direction::all()
        .into_iter()
        .filter(|&dir| {
            scan.at(Vec2::new_from_rotation(dir - facing, 1))
                .unwrap()
                .is_empty()
        })
        .collect::<heapless::Vec<Direction, 4>>();
    available_directions.choose(rng).copied()
}
