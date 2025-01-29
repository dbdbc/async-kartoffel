use crate::{Coords, Direction, Distance, Global, Rotation};

pub trait DistanceMeasure: Clone + 'static {
    fn measure<C: Coords>(dist: Distance<C>) -> u16;
}

#[derive(Clone)]
pub enum DistMax {}
impl DistanceMeasure for DistMax {
    fn measure<C: Coords>(dist: Distance<C>) -> u16 {
        let (d0, d1) = dist.to_generic();
        let (d0, d1) = (d0.unsigned_abs(), d1.unsigned_abs());
        d0.max(d1)
    }
}
#[derive(Clone)]
pub enum DistMin {}
impl DistanceMeasure for DistMin {
    fn measure<C: Coords>(dist: Distance<C>) -> u16 {
        let (d0, d1) = dist.to_generic();
        let (d0, d1) = (d0.unsigned_abs(), d1.unsigned_abs());
        d0.min(d1)
    }
}
#[derive(Clone)]
pub enum DistManhattan {}
impl DistanceMeasure for DistManhattan {
    fn measure<C: Coords>(dist: Distance<C>) -> u16 {
        let (d0, d1) = dist.to_generic();
        let (d0, d1) = (d0.unsigned_abs(), d1.unsigned_abs());
        d0 + d1
    }
}
#[derive(Clone)]
pub enum DistBotWalk {}
impl DistanceMeasure for DistBotWalk {
    fn measure<C: Coords>(dist: Distance<C>) -> u16 {
        let (d0, d1) = dist.to_generic();
        let (d0, d1) = (d0.unsigned_abs(), d1.unsigned_abs());
        let (max, min) = if d0 >= d1 { (d0, d1) } else { (d1, d0) };
        if min == 0 {
            max * 2 // forward
        } else {
            max * 2 + 1 + min * 2 // forward, turn, forward
        }
    }
}
#[derive(Clone)]
pub enum DistBotStab {}
impl DistanceMeasure for DistBotStab {
    fn measure<C: Coords>(dist: Distance<C>) -> u16 {
        let (d0, d1) = dist.to_generic();
        let (d0, d1) = (d0.unsigned_abs(), d1.unsigned_abs());
        let (max, min) = if d0 >= d1 { (d0, d1) } else { (d1, d0) };
        if min == 0 {
            (max * 2).saturating_sub(4) // forward
        } else {
            (max * 2 + 1 + min * 2).saturating_sub(4) // forward, turn, forward
        }
    }
}

pub fn dist_walk_with_rotation(dist: Distance<Global>, facing: Direction) -> u16 {
    // additional initial rotations not covered by DistBotWalk
    let n_rotations = match (
        dist.get(facing),
        dist.get(facing + Rotation::Left).unsigned_abs(),
    ) {
        (..0, 0) => 2,
        (..0, 1..) => 1,
        (0, 0) => 0,
        (0, 1..) => 1,
        (1.., _) => 0,
    };

    n_rotations + DistBotWalk::measure(dist)
}
