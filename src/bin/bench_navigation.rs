#![no_main]
#![no_std]
#![feature(custom_test_frameworks)]
#![test_runner(test_kartoffel::runner)]

use alloc::{boxed::Box, string::ToString};
use async_kartoffel::{print, println, Duration, Position, Timer, Vec2};

use async_algorithm::{ChunkMapHash, ChunkTerrain, Map, Navigation, StatsDog, Terrain};
use core::ops::Deref;
use core::{num::NonZeroU16, ops::RangeInclusive};
use embassy_executor::{task, Executor};
use embassy_futures::select::{select, Either};
use static_cell::StaticCell;

extern crate alloc;

#[no_mangle]
fn main() {
    static EXECUTOR: StaticCell<Executor> = StaticCell::new();
    let executor = EXECUTOR.init(Executor::new());

    println!("navigation bench");

    executor.run(|spawner| {
        spawner.spawn(nav()).unwrap();
    })
}

fn print_map(
    map: &impl Map<Terrain>,
    range_east: RangeInclusive<i16>,
    range_north: RangeInclusive<i16>,
    markers: impl Fn(Position) -> Option<char>,
) {
    for north in range_north.rev() {
        for east in range_east.clone() {
            let pos_print = Position::default() + Vec2::new_global(east, north);
            let ch = match markers(pos_print) {
                Some(ch) => ch,
                None => match map.get(pos_print) {
                    Some(Terrain::Reachable) => ' ',
                    Some(Terrain::Walkable) => '.',
                    Some(Terrain::Blocked) => '█',
                    _ => '░',
                },
            };
            print!("{}", ch);
        }
        println!("");
    }
}

type MyMap = ChunkMapHash<128, Terrain, ChunkTerrain>;
type MyNav = Navigation<ChunkMapHash<64, Option<NonZeroU16>, [[Option<NonZeroU16>; 8]; 8]>, 128>;

struct MapDef<T: Map<Terrain>> {
    map: Box<T>,
    start: Position,
    start_alternative: Option<Position>,
    target: Position,
    range_east: RangeInclusive<i16>,
    range_north: RangeInclusive<i16>,
}

fn make_map<T: Map<Terrain> + Default>(map_string: &str) -> Result<MapDef<T>, &'static str> {
    let mut map: Box<T> = Default::default();
    let mut start = None;
    let mut start_alternative = None;
    let mut target = None;
    let mut pos = Position::default();
    let east = Vec2::new_east(1);
    let south = Vec2::new_south(1);
    let mut east_max = 0i16;
    let mut south_max = 0i16;
    fn reset_east(pos: &mut Position) {
        let east = (*pos - Position::default()).east();
        *pos -= Vec2::new_east(east);
    }
    for c in map_string.chars() {
        match c {
            '\n' => {
                pos += south;
                south_max += 1;
                reset_east(&mut pos);
            }
            ' ' => {
                east_max = east_max.max((pos - Position::default()).east());
                map.set(pos, Terrain::Walkable).unwrap();
                pos += east;
            }
            '█' => {
                east_max = east_max.max((pos - Position::default()).east());
                map.set(pos, Terrain::Blocked).unwrap();
                pos += east;
            }
            '@' => {
                east_max = east_max.max((pos - Position::default()).east());
                map.set(pos, Terrain::Walkable).unwrap();
                if start.is_some() {
                    return Err("start (@) must only appear once");
                }
                start = Some(pos);
                pos += east;
            }
            'a' => {
                east_max = east_max.max((pos - Position::default()).east());
                map.set(pos, Terrain::Walkable).unwrap();
                if start_alternative.is_some() {
                    return Err("alternative start (a) must only appear once");
                }
                start_alternative = Some(pos);
                pos += east;
            }
            'x' => {
                east_max = east_max.max((pos - Position::default()).east());
                map.set(pos, Terrain::Walkable).unwrap();
                if target.is_some() {
                    return Err("target (x) must only appear once");
                }
                target = Some(pos);
                pos += east;
            }
            _ => {
                return Err("allowed chars are '@', 'a', 'x', ' ', '█', '\\n'");
            }
        }
    }
    match (start, target) {
        (Some(start), Some(target)) => Ok(MapDef {
            map,
            start,
            start_alternative,
            target,
            range_east: 0..=east_max,
            range_north: -south_max..=0,
        }),
        _ => Err("start (@) and target (x) must both be defined"),
    }
}

#[task]
async fn nav() -> ! {
    let MapDef {
        map,
        start,
        start_alternative,
        target,
        range_east,
        range_north,
    } = make_map::<MyMap>(MAP_BIG).unwrap();
    println!("{:?}\n{:?}", range_east, range_north);

    print_map(
        map.deref(),
        range_east.clone(),
        range_north.clone(),
        |pos| {
            if pos == start {
                Some('@')
            } else if pos == target {
                Some('x')
            } else if Some(pos) == start_alternative {
                Some('a')
            } else {
                None
            }
        },
    );

    let mut nav: Box<MyNav> = Default::default();
    println!("nav alloc");
    nav.initialize(start, target);

    let walkable = |pos| map.get(pos).is_some_and(|t| t.is_known_walkable());
    let mut dog = StatsDog::new();
    loop {
        let res = dog
            .benchmark(select(
                Timer::after(Duration::from_ticks(5_000)),
                nav.run(walkable),
            ))
            .await;
        // println!("nav state: {:?}", nav.get_state());
        // println!("{:?}", nav.n_active());
        print_map(
            map.deref(),
            range_east.clone(),
            range_north.clone(),
            |pos| {
                if pos == nav.get_state().task().unwrap().from {
                    Some('@')
                } else if pos == target {
                    Some('x')
                } else {
                    nav.get_dist_at(pos)
                        .map(|dist| dist.to_string().chars().last().unwrap())
                }
            },
        );
        if matches!(res, Either::Second(_)) {
            break;
        }
        if let Some(alternative) = start_alternative {
            let new_start = if nav.get_state().task().unwrap().from == alternative {
                start
            } else {
                alternative
            };
            dog.restart_timer();
            nav.update_start(new_start).unwrap();
            dog.feed();
        }
    }
    println!("{}", dog);

    #[allow(clippy::empty_loop)]
    loop {}
}

#[allow(unused)]
const MAP_SMALL: &str = "█████████████
█       ██ a█
█    ██     █
█     █ ██  █
█ █████  █ ██
█ █     █@ ██
█ █ ███ ██ ██
█ █   █    ██
█ ██  █████ █
█x█    █    █
█   █       █
█████████████";

#[allow(unused)]
const MAP_MEDIUM: &str = "████████████████████████
█       ██  ██         █
████ ██   @    ██ ██  ██
█     █ ██  █ ██   █   █
█ █████  █ █   █   █████
█ █   █    █ ████ ████ █
█ █ █ █████       █    █
█ █ █  █    ███████x ███
█   █  █ ██         █  █
████████████████████████";

#[allow(unused)]
const MAP_BIG: &str = "████████████████████████
█       ██  ██         █
████ ██        ██ ████ █
█     █ ██  █ ██   █a  █
█ █████  █ █   █   █████
█ █     █  █   ████    █
█ █ ███ ██ ██     █  █ █
█ █   █    █ ████ ████ █
█ █ █ █████       █    █
█ █ █  █    ███████x ███
█   █  █ ███        █  █
█@   ███ ██ ██  ███ █  █
██  █    █   ██ █    █ █
██ ██    ██  █         █
█  ██    █   ████    ███
█ █ █    ██  █ ██      █
█ █ █        ██  █████ █
█   █ ███ ████   █     █
█ █ █ █     █          █
█ █ █ ███████ █        █
█   █                  █
████████████████████████";

#[allow(unused)]
const MAP_DIAG: &str = "████████████████████████
█@█                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█   █                 x█
████████████████████████";

#[allow(unused)]
const MAP_DIAG_ALT: &str = "████████████████████████
█@█                    █
█ █                    █
█ █                    █
█a█                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█ █                    █
█   █                 x█
████████████████████████";
