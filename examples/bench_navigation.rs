#![no_main]
#![no_std]

use alloc::{boxed::Box, string::ToString};
use async_kartoffel::{
    algorithm::{isqrt, Breakpoint, ChunkMap, ChunkTerrain, Map, Navigation, Terrain},
    print, println, Distance, Duration, Instant, Position, Timer,
};
use core::future::Future;
use core::ops::Deref;
use core::{fmt::Display, num::NonZeroU16, ops::RangeInclusive};
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

pub struct StatsDog {
    sum_duration: Duration,
    counter: u32,
    max_duration: Duration,
    min_duration: Duration,
    sum_sq_duration: u64,
    last_time: Instant,
}
impl StatsDog {
    fn new() -> Self {
        Self {
            last_time: Instant::now(),
            sum_duration: Default::default(),
            counter: 0,
            max_duration: Default::default(),
            min_duration: Default::default(),
            sum_sq_duration: 0,
        }
    }
    /// benchmark latency for an async function, only valid if there a no other futures running
    async fn benchmark<F: Future>(&mut self, f: F) -> F::Output {
        self.restart_timer();
        match select(f, self.feed_continuous()).await {
            Either::First(output) => {
                self.feed();
                output
            }
            Either::Second(_) => unreachable!(),
        }
    }
    /// runs indefinitely, to be used with select
    async fn feed_continuous(&mut self) -> ! {
        loop {
            Breakpoint::new().await;
            self.feed();
        }
    }
    fn feed(&mut self) {
        let now = Instant::now();
        let duration = (now - self.last_time).unwrap();

        self.sum_duration += duration;
        self.counter += 1;
        self.max_duration = self.max_duration.max(duration);
        self.min_duration = self.min_duration.min(duration);
        self.sum_sq_duration += u64::from(duration.as_ticks()).pow(2);

        self.restart_timer();
    }
    fn restart_timer(&mut self) {
        self.last_time = Instant::now();
    }
    fn mean(&self) -> u32 {
        self.sum_duration.as_ticks() / self.counter
    }
    fn std(&self) -> u32 {
        // std = 1 / (N - 1) * sum((x - µ)^2)
        //     = 1 / (N - 1) * (sum(x^2) - 2 sum(x) µ + N µ^2)
        //     = 1 / (N - 1) * (sum(x^2) - N µ^2)
        isqrt(
            (self.sum_sq_duration
                - u64::from(self.sum_duration.as_ticks()).pow(2) / u64::from(self.counter))
                / (u64::from(self.counter) - 1),
        ) as u32
    }
}
impl Display for StatsDog {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "latency stats")?;
        writeln!(f, "n    : {}", self.counter)?;
        writeln!(f, "total: {}", self.sum_duration.as_ticks())?;
        writeln!(f, "min  : {}", self.min_duration.as_ticks())?;
        writeln!(f, "mean : {}", self.mean())?;
        writeln!(f, "max  : {}", self.max_duration.as_ticks())?;
        writeln!(f, "std  : {}", self.std())
    }
}

fn print_map(
    map: &impl Map<Terrain>,
    range_east: RangeInclusive<i16>,
    range_north: RangeInclusive<i16>,
    markers: impl Fn(Position) -> Option<char>,
) {
    for north in range_north.rev() {
        for east in range_east.clone() {
            let pos_print = Position::default() + Distance::new_global(east, north);
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

type MyMap = ChunkMap<128, Terrain, ChunkTerrain>;
type MyNav = Navigation<ChunkMap<64, Option<NonZeroU16>, [[Option<NonZeroU16>; 8]; 8]>, 128>;

fn make_map<T: Map<Terrain> + Default>(
    map_string: &str,
) -> Result<
    (
        Box<T>,
        Position,
        Option<Position>,
        Position,
        RangeInclusive<i16>,
        RangeInclusive<i16>,
    ),
    &'static str,
> {
    let mut map: Box<T> = Default::default();
    let mut start = None;
    let mut alternative = None;
    let mut target = None;
    let mut pos = Position::default();
    let east = Distance::new_east(1);
    let south = Distance::new_south(1);
    let mut east_max = 0i16;
    let mut south_max = 0i16;
    fn reset_east(pos: &mut Position) {
        let east = (*pos - Position::default()).east();
        *pos -= Distance::new_east(east);
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
                if alternative.is_some() {
                    return Err("alternative start (a) must only appear once");
                }
                alternative = Some(pos);
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
        (Some(start), Some(target)) => Ok((
            map,
            start,
            alternative,
            target,
            0..=east_max,
            -south_max..=0,
        )),
        _ => Err("start (@) and target (x) must both be defined"),
    }
}

#[task]
async fn nav() -> ! {
    let (map, start, alternative, target, range_east, range_north) =
        make_map::<MyMap>(MAP_DIAG_ALT).unwrap();
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
            } else if Some(pos) == alternative {
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
        // print_map(
        //     map.deref(),
        //     range_east.clone(),
        //     range_north.clone(),
        //     |pos| {
        //         if pos == nav.get_state().task().unwrap().from {
        //             Some('@')
        //         } else if pos == target {
        //             Some('x')
        //         } else if let Some(dist) = nav.get_dist_at(pos) {
        //             Some(dist.to_string().chars().last().unwrap())
        //         } else {
        //             None
        //         }
        //     },
        // );
        if matches!(res, Either::Second(_)) {
            break;
        }
        if let Some(alternative) = alternative {
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
