use crate::{Coords, Distance};

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
