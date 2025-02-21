#![no_std]

use core::{
    fmt::{self, Display},
    hash::Hasher,
    ops::Range,
};

use phf_shared::{FmtConst, PhfBorrow, PhfHash};

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Clone)]
pub struct Chunk<const N: usize>(pub [[bool; N]; N]);

impl<const N: usize> Default for Chunk<N> {
    fn default() -> Self {
        Chunk([[false; N]; N])
    }
}

impl<const N: usize> Chunk<N> {
    pub fn ranges(height: usize, width: usize) -> (Range<usize>, Range<usize>) {
        let r = N / 2;
        assert!(N == r * 2 + 1);
        (r..height - r, r..width - r)
    }
    pub fn center(&self) -> bool {
        let r = N / 2;
        assert!(N == r * 2 + 1);
        self.0[r][r]
    }
}

impl<const N: usize> FmtConst for Chunk<N> {
    fn fmt_const(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "::kartoffel_gps::Chunk::<{}>({:?})", N, self.0)
    }
}
impl<const N: usize> PhfHash for Chunk<N> {
    fn phf_hash<H: Hasher>(&self, state: &mut H) {
        for i in 0..N {
            self.0[i].phf_hash(state);
        }
    }
}
impl<const N: usize> PhfBorrow<Chunk<N>> for Chunk<N> {
    fn borrow(&self) -> &Chunk<N> {
        self
    }
}

impl<const N: usize> Display for Chunk<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for line in self.0 {
            for val in line {
                write!(f, "{}", if val { "." } else { "#" })?;
            }
            writeln!(f, "")?;
        }
        Ok(())
    }
}
