#[derive(PartialEq, Clone, Ord, PartialOrd, Eq, Debug, Copy, Hash)]
pub enum Tile {
    Bot,
    Empty,
    Void,
    WallHorizontal,
    WallVertical,
    WallCave,
    EntryExit,
    Diamond,
    Flag,
}

impl Tile {
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '@' => Some(Self::Bot),
            '.' => Some(Self::Empty),
            ' ' => Some(Self::Void),
            '|' => Some(Self::WallVertical),
            '-' => Some(Self::WallHorizontal),
            '+' => Some(Self::EntryExit),
            '*' => Some(Self::Diamond),
            '=' => Some(Self::Flag),
            '#' => Some(Self::WallCave),
            _ => None,
        }
    }
    pub const fn to_char(self) -> char {
        match self {
            Self::Bot => '@',
            Self::Empty => '.',
            Self::Void => ' ',
            Self::WallVertical => '|',
            Self::WallHorizontal => '-',
            Self::WallCave => '#',
            Self::EntryExit => '+',
            Self::Diamond => '*',
            Self::Flag => '=',
        }
    }
    /// The "." tile. You can go there.
    pub const fn is_empty(self) -> bool {
        matches!(self, Self::Empty)
    }
    /// Right now it might be blocked by a bot or an item, but in principle you can go there.
    pub const fn is_walkable_terrain(self) -> bool {
        matches!(
            self,
            Self::Empty | Self::Bot | Self::EntryExit | Self::Diamond | Self::Flag
        )
    }
    pub const fn is_bot(self) -> bool {
        matches!(self, Self::Bot)
    }
    pub const fn is_item(self) -> bool {
        matches!(self, Self::Diamond | Self::Flag)
    }
}
