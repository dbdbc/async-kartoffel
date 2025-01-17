#[derive(PartialEq, Clone, Ord, PartialOrd, Eq, Debug, Copy, Hash)]
pub enum Tile {
    Bot,
    Empty,
    Void,
    WallHorizontal,
    WallVertical,
    EntryExit,
    Diamond,
    Flag,
}

impl Tile {
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '@' => Some(Tile::Bot),
            '.' => Some(Tile::Empty),
            ' ' => Some(Tile::Void),
            '|' => Some(Tile::WallVertical),
            '-' => Some(Tile::WallHorizontal),
            '+' => Some(Tile::EntryExit),
            '*' => Some(Tile::Diamond),
            '=' => Some(Tile::Flag),
            _ => None,
        }
    }
    pub const fn to_char(self) -> char {
        match self {
            Tile::Bot => '@',
            Tile::Empty => '.',
            Tile::Void => ' ',
            Tile::WallVertical => '|',
            Tile::WallHorizontal => '-',
            Tile::EntryExit => '+',
            Tile::Diamond => '*',
            Tile::Flag => '=',
        }
    }
    pub const fn is_empty(self) -> bool {
        matches!(self, Tile::Empty)
    }
    pub const fn is_walkable_terrain(self) -> bool {
        matches!(
            self,
            Tile::Empty | Tile::Bot | Tile::EntryExit | Tile::Diamond | Tile::Flag
        )
    }
    pub const fn is_bot(self) -> bool {
        matches!(self, Tile::Bot)
    }
    pub const fn is_item(self) -> bool {
        matches!(self, Tile::Diamond | Tile::Flag)
    }
}
