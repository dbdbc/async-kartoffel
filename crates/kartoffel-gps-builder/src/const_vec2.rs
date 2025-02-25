use core::fmt::Display;

use async_kartoffel::{Global, Vec2};

pub struct ArrayBuilder<'a>(pub &'a [Vec2<Global>]);

impl ArrayBuilder<'_> {
    pub fn type_string(&self) -> String {
        format!(
            "[::async_kartoffel::Vec2<::async_kartoffel::Global>; {}]",
            self.0.len()
        )
    }
}

impl Display for ArrayBuilder<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[\n",)?;
        for vec in self.0 {
            write!(
                f,
                "    ::async_kartoffel::Vec2::new_global({}, {}),\n",
                vec.east(),
                vec.north()
            )?;
        }
        write!(f, "]")?;
        Ok(())
    }
}
