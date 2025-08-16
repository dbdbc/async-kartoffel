use async_kartoffel_generic::{Coords, Direction, Global, Rotation, Vec2};

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
/// how long does it take for the bot to walk this distance, in 5k clock cycles, minimum
pub enum DistanceBotWalk {}
impl DistanceMeasure for DistanceBotWalk {
    fn measure<C: Coords>(vec: Vec2<C>) -> u16 {
        let (d0, d1) = vec.to_generic();
        let (d0, d1) = (d0.unsigned_abs(), d1.unsigned_abs());
        let (max, min) = if d0 >= d1 { (d0, d1) } else { (d1, d0) };
        if min == 0 {
            max * 4 // forward
        } else {
            max * 4 + 5 + min * 4 // forward, turn, forward
        }
    }
}
#[derive(Clone)]
/// how long does it take for the bot to stab a bot this far away, in 5k clock cycles, minimum
pub enum DistanceBotStab {}
impl DistanceMeasure for DistanceBotStab {
    fn measure<C: Coords>(vec: Vec2<C>) -> u16 {
        let (d0, d1) = vec.to_generic();
        let (d0, d1) = (d0.unsigned_abs(), d1.unsigned_abs());
        let (max, min) = if d0 >= d1 { (d0, d1) } else { (d1, d0) };
        if min == 0 {
            (max * 4).saturating_sub(8) // forward
        } else {
            (max * 4 + 5 + min * 4).saturating_sub(8) // forward, turn, forward
        }
    }
}

/// how many 5k clock cycles does a bot facing a certain direction require to walk to vec
/// moving backwards is not accounted for
pub fn distance_walk_with_rotation(vec: Vec2<Global>, facing: Direction) -> u16 {
    // additional initial rotations not covered by DistBotWalk
    let n_rotations = match (
        vec.in_direction(facing),
        vec.in_direction(facing + Rotation::Left).unsigned_abs(),
    ) {
        (..0, 0) => 2,
        (..0, 1..) => 1,
        (0, 0) => 0,
        (0, 1..) => 1,
        (1.., _) => 0,
    };

    5 * n_rotations + DistanceBotWalk::measure(vec)
}
