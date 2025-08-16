use async_kartoffel_generic::Position;

pub trait Map<T> {
    fn set(&mut self, pos: Position, t: T) -> Result<(), T>;
    fn get(&self, pos: Position) -> Option<T>;
    fn clear(&mut self);
}
