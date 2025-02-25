use core::{fmt::Display, ops::Range};

#[derive(Debug)]
pub struct ConstSparseGraphNode {
    pub start: u16,
    pub mid: u16,
    pub end: u16,
}
impl ConstSparseGraphNode {
    fn range_before(&self) -> Range<usize> {
        usize::from(self.start)..usize::from(self.mid)
    }
    fn range_after(&self) -> Range<usize> {
        usize::from(self.mid)..usize::from(self.end)
    }
}
impl Display for ConstSparseGraphNode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "::kartoffel_gps::const_graph::{:?}", self)
    }
}

pub struct ConstSparseGraph<const N_NODES: usize, const N_STORE: usize> {
    pub nodes: [ConstSparseGraphNode; N_NODES],
    pub data: [u16; N_STORE],
}
impl<const N_NODES: usize, const N_STORE: usize> ConstSparseGraph<N_NODES, N_STORE> {
    pub fn after(&self, index: u16) -> &[u16] {
        &self.data[self.nodes[usize::from(index)].range_after()]
    }
    pub fn before(&self, index: u16) -> &[u16] {
        &self.data[self.nodes[usize::from(index)].range_before()]
    }
    pub fn size(&self) -> u16 {
        u16::try_from(N_NODES).unwrap()
    }
}
