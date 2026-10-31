use crate::model::{Passage, PassageCluster, PassageEmbedding};
use std::collections::{BTreeMap, HashMap};

pub fn cluster_passages(
    passages: &[Passage],
    embeddings: &[PassageEmbedding],
) -> Vec<PassageCluster> {
    if passages.is_empty() {
        return Vec::new();
    }
    let mut union_find = UnionFind::new(passages.len());
    let index_by_id = passages
        .iter()
        .enumerate()
        .map(|(index, passage)| (passage.id.as_str(), index))
        .collect::<HashMap<_, _>>();

    for (left_index, left) in embeddings.iter().enumerate() {
        for right in embeddings.iter().skip(left_index + 1) {
            let Some(&right_index) = index_by_id.get(right.passage_id.as_str()) else {
                continue;
            };
            let similarity = cosine(&left.vector, &right.vector);
            let same_paragraph =
                passages[left_index].paragraph_index == passages[right_index].paragraph_index;
            if similarity >= 0.74 || (same_paragraph && similarity >= 0.58) {
                union_find.union(left_index, right_index);
            }
        }
    }

    let mut grouped: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for index in 0..passages.len() {
        grouped
            .entry(union_find.find(index))
            .or_default()
            .push(index);
    }

    grouped
        .into_values()
        .enumerate()
        .map(|(cluster_index, indices)| {
            let representative = indices[0];
            let passage_ids = indices
                .iter()
                .map(|&index| passages[index].id.clone())
                .collect::<Vec<_>>();
            PassageCluster {
                id: format!("cluster_{}", cluster_index + 1),
                passage_ids,
                centroid_vector_id: format!("centroid_{}", cluster_index + 1),
                representative_passage_id: passages[representative].id.clone(),
                confidence: if indices.len() > 1 { 0.72 } else { 0.5 },
            }
        })
        .collect()
}

fn cosine(left: &[f32], right: &[f32]) -> f64 {
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return 0.0;
    }
    let dot = left
        .iter()
        .zip(right)
        .map(|(left, right)| f64::from(*left) * f64::from(*right))
        .sum::<f64>();
    let left_norm = left
        .iter()
        .map(|value| f64::from(*value) * f64::from(*value))
        .sum::<f64>()
        .sqrt();
    let right_norm = right
        .iter()
        .map(|value| f64::from(*value) * f64::from(*value))
        .sum::<f64>()
        .sqrt();
    if left_norm == 0.0 || right_norm == 0.0 {
        0.0
    } else {
        dot / (left_norm * right_norm)
    }
}

#[derive(Debug, Clone)]
struct UnionFind {
    parents: Vec<usize>,
}

impl UnionFind {
    fn new(size: usize) -> Self {
        Self {
            parents: (0..size).collect(),
        }
    }

    fn find(&mut self, index: usize) -> usize {
        let parent = self.parents[index];
        if parent == index {
            index
        } else {
            let root = self.find(parent);
            self.parents[index] = root;
            root
        }
    }

    fn union(&mut self, left: usize, right: usize) {
        let left_root = self.find(left);
        let right_root = self.find(right);
        if left_root != right_root {
            self.parents[right_root] = left_root;
        }
    }
}
