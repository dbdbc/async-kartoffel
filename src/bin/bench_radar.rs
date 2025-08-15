#![no_main]
#![no_std]
#![feature(custom_test_frameworks)]
#![test_runner(test_kartoffel::runner)]

use core::sync::atomic::{compiler_fence, Ordering};

use async_kartoffel::{println, Bot, Instant, RadarSize, Tile, Vec2, D3, D5, D7, D9};
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

fn with_timing<T1, T2>(msg: &str, t1: T1, closure: impl FnOnce(T1) -> T2) -> T2 {
    let start = Instant::now();
    compiler_fence(Ordering::AcqRel);
    let t2 = closure(t1);
    compiler_fence(Ordering::AcqRel);
    let end = Instant::now();
    println!("{}: {:?}", msg, (end - start).unwrap());
    t2
}

struct Range2d {
    start0: i8,
    start1: i8,
    size1: i8,
    index: i8,
    index_end: i8,
}
impl Range2d {
    fn from_radar<Size: RadarSize>() -> Self {
        Self::new(
            (-(Size::R as i8), -(Size::R as i8)),
            (Size::R as i8 + 1, Size::R as i8 + 1),
        )
        .unwrap()
    }
    fn new(start: (i8, i8), end: (i8, i8)) -> Option<Self> {
        let index_end =
            (i32::from(end.0) - i32::from(start.0)) * (i32::from(end.1) - i32::from(start.0));
        if end.0 < start.0 || end.1 < start.1 || index_end >= i32::from(i8::MAX) {
            None
        } else {
            Some(Self {
                start0: start.0,
                start1: start.1,
                size1: end.1 - start.1,
                index: 0,
                index_end: index_end as i8,
            })
        }
    }
}
impl Iterator for Range2d {
    type Item = (i8, i8);

    fn next(&mut self) -> Option<Self::Item> {
        let ret = (
            self.start0 + self.index.div_euclid(self.size1),
            self.start1 + self.index.rem_euclid(self.size1),
        );
        self.index += 1;
        if self.index <= self.index_end {
            Some(ret)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        todo!()
    }
}

async fn bench<Size: RadarSize>(bot: &mut Bot) {
    let mut count_all = 0;
    println!("{}", Size::to_str());
    count_all += {
        let scan = bot.radar.scan::<Size>().await;
        with_timing("tile", (), |_| scan.iter_tile(Tile::Bot).count())
    };
    count_all += {
        let scan = bot.radar.scan::<Size>().await;
        with_timing("iter", (), |_| {
            scan.iter().filter(|&(_, tile)| tile == Tile::Bot).count()
        })
    };
    count_all += {
        let scan = bot.radar.scan::<Size>().await;
        with_timing("fc2d", (), |_| {
            let mut count = 0;
            for (i1, i2) in Range2d::from_radar::<Size>() {
                if scan.at(Vec2::new_front_right(i1.into(), i2.into())) == Some(Tile::Bot)
                    && (i1 != 0 || i2 != 0)
                {
                    count += 1;
                }
            }
            count
        })
    };
    count_all += {
        let scan = bot.radar.scan::<Size>().await;
        with_timing("fc  ", (), |_| {
            let mut count = 0;
            for i1 in Size::range() {
                for i2 in Size::range() {
                    if scan.at(Vec2::new_front_right(i1.into(), i2.into())) == Some(Tile::Bot)
                        && (i1 != 0 || i2 != 0)
                    {
                        count += 1;
                    }
                }
            }
            count
        })
    };
    count_all += {
        let scan = bot.radar.scan::<Size>().await;
        with_timing("fu2d", (), |_| {
            let mut count = 0;
            for (i1, i2) in Range2d::from_radar::<Size>() {
                if scan.at_unchecked(i1, i2) == Tile::Bot.to_char() && (i1 != 0 || i2 != 0) {
                    count += 1;
                }
            }
            count
        })
    };
    count_all += {
        let scan = bot.radar.scan::<Size>().await;
        with_timing("fu  ", (), |_| {
            let mut count = 0;
            for i1 in Size::range() {
                for i2 in Size::range() {
                    if scan.at_unchecked(i1, i2) == Tile::Bot.to_char() && (i1 != 0 || i2 != 0) {
                        count += 1;
                    }
                }
            }
            count
        })
    };
    {
        let scan = bot.radar.scan::<Size>().await;
        let mut slice = [0 as char; 81];
        with_timing("st  ", (), |_| {
            for i1 in Size::range() {
                for i2 in Size::range() {
                    slice[((i1 + (Size::R as i8)) * (2 * Size::R as i8 + 1) + i2 + (Size::R as i8))
                        as usize] = scan.at_unchecked(i1, i2);
                }
            }
        });
        let sum = slice.iter().map(|&c| c as u32).sum::<u32>();
        let count = slice.iter().filter(|&&c| c as u32 != 0).count();
        println!("{}, {}", sum, count);
    }
    {
        let scan = bot.radar.scan::<Size>().await;
        let mut slice = [0 as char; 81];
        with_timing("st2d", (), |_| {
            for (i1, i2) in Range2d::from_radar::<Size>() {
                slice[((i1 + (Size::R as i8)) * (2 * Size::R as i8 + 1) + i2 + (Size::R as i8))
                    as usize] = scan.at_unchecked(i1, i2);
            }
        });
        let sum = slice.iter().map(|&c| c as u32).sum::<u32>();
        let count = slice.iter().filter(|&&c| c as u32 != 0).count();
        println!("{}, {}", sum, count);
    }
    {
        let scan = bot.radar.scan::<Size>().await;
        let mut slice = [[0 as char; 9]; 9];
        with_timing("2sto", (), |_| {
            for i1 in Size::range() {
                for i2 in Size::range() {
                    slice[(i1 + 4) as usize][(i2 + 4) as usize] = scan.at_unchecked(i1, i2);
                }
            }
        });
        let sum = slice
            .iter()
            .flat_map(|r| r.iter())
            .map(|&c| c as u32)
            .sum::<u32>();
        let count = slice
            .iter()
            .flat_map(|r| r.iter())
            .filter(|&&c| c as u32 != 0)
            .count();
        println!("{}, {}", sum, count);
    }
    {
        let scan = bot.radar.scan::<Size>().await;
        let mut bots = heapless::Vec::<(i8, i8), 81>::new();
        with_timing("bots", (), |_| {
            for i1 in Size::range() {
                for i2 in Size::range() {
                    if scan.at_unchecked(i1, i2) == Tile::Bot.to_char() {
                        bots.push((i1, i2)).unwrap();
                    }
                }
            }
        });
        println!("{}", bots.iter().count());
    }
    println!("done {}", count_all);
}

#[task]
async fn main_task(mut bot: Bot) -> ! {
    loop {
        bench::<D3>(&mut bot).await;
        bench::<D5>(&mut bot).await;
        bench::<D7>(&mut bot).await;
        bench::<D9>(&mut bot).await;
    }
}
