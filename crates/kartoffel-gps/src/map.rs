use async_kartoffel::{Global, Vec2};

use crate::GlobalPos;

pub trait TrueMap: Send + Sync + 'static {
    /// whether terrain at [`pos`] is walkable
    fn get(&self, pos: GlobalPos) -> bool;
    fn vec_east(&self) -> Vec2<Global>;
    fn vec_south(&self) -> Vec2<Global>;
    fn width(&self) -> u16;
    fn height(&self) -> u16;
}

pub struct TrueMapImpl<const WIDTH: i16, const HEIGHT: i16, const STORE: usize>(pub [u8; STORE]);

impl<const WIDTH: i16, const HEIGHT: i16, const STORE: usize> TrueMap
    for TrueMapImpl<WIDTH, HEIGHT, STORE>
{
    fn get(&self, pos: GlobalPos) -> bool {
        let vec = pos.subtract_anchor();
        let east = vec.east();
        let south = vec.south();
        if east < 0 || east >= WIDTH || south < 0 || south >= HEIGHT {
            false
        } else {
            let index = usize::try_from(east + south * WIDTH).unwrap();
            (self.0[index.div_euclid(8)] & (1u8 << index.rem_euclid(8))) != 0u8
        }
    }

    fn vec_east(&self) -> Vec2<Global> {
        Vec2::new_east(WIDTH)
    }

    fn vec_south(&self) -> Vec2<Global> {
        Vec2::new_south(HEIGHT)
    }

    fn width(&self) -> u16 {
        WIDTH.try_into().unwrap()
    }

    fn height(&self) -> u16 {
        HEIGHT.try_into().unwrap()
    }
}
