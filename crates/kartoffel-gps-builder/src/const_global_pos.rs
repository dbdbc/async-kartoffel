use core::fmt::Display;

use kartoffel_gps::GlobalPos;

pub struct ArrayBuilder<'a>(pub &'a [GlobalPos]);

impl ArrayBuilder<'_> {
    pub fn type_string(&self) -> String {
        format!("[::kartoffel_gps::GlobalPos; {}]", self.0.len())
    }
}

impl Display for ArrayBuilder<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[\n",)?;
        for pos in self.0 {
            let vec = pos.sub_anchor();
            write!(
                f,
                "    ::kartoffel_gps::GlobalPos::add_to_anchor(::async_kartoffel::Vec2::new_global({}, {})),\n",
                vec.east(),
                vec.north()
            )?;
        }
        write!(f, "]")?;
        Ok(())
    }
}
