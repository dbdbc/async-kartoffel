use core::{fmt::Display, iter::Iterator};

use kartoffel_gps::const_graph::ConstSparseGraphNode;
use ndarray::{Array2, Axis};

#[derive(Default)]
pub struct ConstSparseGraphBuilder {
    nodes: Vec<ConstSparseGraphNode>,
    data: Vec<u16>,
    max_index: Option<u16>,
    offset: usize,
}

impl ConstSparseGraphBuilder {
    pub fn from_matrix(mat: &Array2<u32>) -> Self {
        let n = mat.len_of(Axis(0));
        assert!(mat.len_of(Axis(1)) == n);

        let mut builder = Self::default();

        for i1 in 0..n {
            let mut before = Vec::new();
            let mut after = Vec::new();
            for i2 in 0..n {
                if mat[(i1, i2)] > 0 {
                    before.push(u16::try_from(i2).unwrap());
                }
                if mat[(i2, i1)] > 0 {
                    after.push(u16::try_from(i2).unwrap());
                }
            }
            builder.add_node(&before, &after);
        }
        builder
    }
    pub fn add_node(&mut self, before: &Vec<u16>, after: &Vec<u16>) {
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
        if let Some(max_index) = self.max_index {
            if usize::from(max_index) >= self.nodes.len() {
                return false;
            }
        }
        if self.nodes.len() > u16::MAX.into() {
            return false;
        }
        return true;
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
            write!(f, "        {},\n", node)?;
        }
        write!(f, "    ],\n    data: [")?;
        for d in &self.data {
            write!(f, "        {},\n", d)?;
        }
        write!(f, "    ]\n}}")?;
        Ok(())
    }
}
