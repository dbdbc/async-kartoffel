use async_kartoffel::{Coords, Direction, Global, Rotation, Vec2};

pub trait DistanceMeasure: Clone + 'static {
    fn measure<C: Coords>(vec: Vec2<C>) -> u16;
}

#[derive(Clone)]
pub enum DistanceMax {}
impl DistanceMeasure for DistanceMax {
    fn measure<C: Coords>(vec: Vec2<C>) -> u16 {
        let (d0, d1) = vec.to_generic();
        let (d0, d1) = (d0.unsigned_abs(), d1.unsigned_abs());
        d0.max(d1)
    }
}
#[derive(Clone)]
pub enum DistanceMin {}
impl DistanceMeasure for DistanceMin {
    fn measure<C: Coords>(vec: Vec2<C>) -> u16 {
        let (d0, d1) = vec.to_generic();
        let (d0, d1) = (d0.unsigned_abs(), d1.unsigned_abs());
        d0.min(d1)
    }
}
#[derive(Clone)]
pub enum DistanceManhattan {}
impl DistanceMeasure for DistanceManhattan {
    fn measure<C: Coords>(vec: Vec2<C>) -> u16 {
        let (d0, d1) = vec.to_generic();
        let (d0, d1) = (d0.unsigned_abs(), d1.unsigned_abs());
        d0 + d1
    }
}
#[derive(Clone)]
pub enum DistanceBotWalk {}
impl DistanceMeasure for DistanceBotWalk {
    fn measure<C: Coords>(vec: Vec2<C>) -> u16 {
        let (d0, d1) = vec.to_generic();
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
pub enum DistanceBotStab {}
impl DistanceMeasure for DistanceBotStab {
    fn measure<C: Coords>(vec: Vec2<C>) -> u16 {
        let (d0, d1) = vec.to_generic();
        let (d0, d1) = (d0.unsigned_abs(), d1.unsigned_abs());
        let (max, min) = if d0 >= d1 { (d0, d1) } else { (d1, d0) };
        if min == 0 {
            (max * 2).saturating_sub(4) // forward
        } else {
            (max * 2 + 1 + min * 2).saturating_sub(4) // forward, turn, forward
        }
    }
}

/// how many 10k clock cycles does a bot looking at facing require to walk to vec
pub fn distance_walk_with_rotation(vec: Vec2<Global>, facing: Direction) -> u16 {
    // additional initial rotations not covered by DistBotWalk
    let n_rotations = match (
        vec.get(facing),
        vec.get(facing + Rotation::Left).unsigned_abs(),
    ) {
        (..0, 0) => 2,
        (..0, 1..) => 1,
        (0, 0) => 0,
        (0, 1..) => 1,
        (1.., _) => 0,
    };

    n_rotations + DistanceBotWalk::measure(vec)
}
