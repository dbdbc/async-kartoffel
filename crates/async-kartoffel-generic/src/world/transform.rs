use crate::{Direction, Local, Position, PositionAnchor, Rotation, Vec2};

/// translation is given in original coordinates, so not rotated yet
#[derive(Default, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Transform {
    vec: Vec2<Local>,
    rotation: Rotation,
}

impl From<Vec2<Local>> for Transform {
    fn from(value: Vec2<Local>) -> Self {
        Self {
            vec: value,
            rotation: Rotation::Id,
        }
    }
}

impl From<Rotation> for Transform {
    fn from(value: Rotation) -> Self {
        Self {
            vec: Vec2::default(),
            rotation: value,
        }
    }
}

impl Transform {
    pub const fn identity() -> Self {
        Self {
            vec: Vec2::zero(),
            rotation: Rotation::Id,
        }
    }

    pub fn translation(&self) -> Vec2<Local> {
        self.vec
    }

    pub fn rotation(&self) -> Rotation {
        self.rotation
    }

    pub fn new(translation: Vec2<Local>, rotation: Rotation) -> Self {
        Self {
            vec: translation,
            rotation,
        }
    }

    pub fn chain(&self, next: Self) -> Self {
        Self {
            vec: self.vec + next.vec.rotate(self.rotation),
            rotation: self.rotation + next.rotation,
        }
    }

    /// self.chain(self.inverse()) == Self::identity()
    /// self.inverse(self.inverse()) == self
    pub fn inverse(&self) -> Self {
        Self {
            vec: (-self.vec).rotate(-self.rotation),
            rotation: -self.rotation,
        }
    }

    pub fn apply<A: PositionAnchor>(
        &self,
        pos: Position<A>,
        facing: Direction,
    ) -> (Position<A>, Direction) {
        (pos + self.vec.global(facing), facing + self.rotation)
    }

    pub fn apply_dir(&self, facing: Direction) -> Direction {
        facing + self.rotation
    }
}
