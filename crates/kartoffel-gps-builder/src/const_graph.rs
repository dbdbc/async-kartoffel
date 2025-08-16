use core::{fmt::Display, iter::Iterator};

use crate::graph::GraphMapping;
use kartoffel_gps::const_graph::ConstSparseGraphNode;

use crate::beacon_nav::PosGraph;

#[derive(Default)]
pub struct ConstSparseGraphBuilder {
    nodes: Vec<ConstSparseGraphNode>,
    data: Vec<u16>,
    max_index: Option<u16>,
    offset: usize,
}

impl ConstSparseGraphBuilder {
    pub fn from_graph(graph: &PosGraph) -> Self {
        assert!(graph.get_map_start().equals(graph.get_map_destination()));
        let n = graph.len_start();

        let mut builder = Self::default();

        for i1 in 0..n {
            let mut before_i1 = Vec::new();
            let mut after_i1 = Vec::new();
            for i2 in 0..n {
                if graph.get(i1, i2).is_some() {
                    after_i1.push(u16::try_from(i2).unwrap());
                }
                if graph.get(i2, i1).is_some() {
                    before_i1.push(u16::try_from(i2).unwrap());
                }
            }
            builder.add_node(&before_i1, &after_i1);
        }
        builder
    }
    pub fn add_node(&mut self, before: &[u16], after: &[u16]) {
        assert!(is_sorted_deduplicated(before));
        assert!(is_sorted_deduplicated(after));

        if let Some(&index) = before.iter().chain(after.iter()).max() {
            if let Some(index2) = self.max_index {
                self.max_index = Some(index.max(index2));
            } else {
                self.max_index = Some(index);
            }
        }
        let node = ConstSparseGraphNode {
            start: u16::try_from(self.offset).unwrap(),
            mid: u16::try_from(self.offset + before.len()).unwrap(),
            end: u16::try_from(self.offset + before.len() + after.len()).unwrap(),
        };
        self.offset += before.len() + after.len();

        self.nodes.push(node);
        self.data.extend_from_slice(before);
        self.data.extend_from_slice(after);
    }

    pub fn type_string(&self) -> String {
        assert!(self.is_valid());
        format!(
            "::kartoffel_gps::const_graph::ConstSparseGraph<{}, {}>",
            self.nodes.len(),
            self.data.len()
        )
    }

    pub fn is_valid(&self) -> bool {
        if let Some(max_index) = self.max_index
            && usize::from(max_index) >= self.nodes.len()
        {
            return false;
        }
        if self.nodes.len() > u16::MAX.into() {
            return false;
        }
        true
    }
}

fn is_sorted_deduplicated(arr: &[u16]) -> bool {
    (0..arr.len() - 1).all(|i| arr[i] < arr[i + 1])
}

impl Display for ConstSparseGraphBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        assert!(self.is_valid());

        write!(
            f,
            "::kartoffel_gps::const_graph::ConstSparseGraph {{\n    nodes: [\n"
        )?;
        for node in &self.nodes {
            writeln!(f, "        {},", node)?;
        }
        write!(f, "    ],\n    data: [")?;
        for d in &self.data {
            write!(f, "{}, ", d)?;
        }
        writeln!(f, "],}}")?;
        Ok(())
    }
}
