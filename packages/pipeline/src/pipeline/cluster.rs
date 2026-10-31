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

    let passage_indices = embeddings
        .iter()
        .map(|embedding| index_by_id.get(embedding.passage_id.as_str()).copied())
        .collect::<Vec<_>>();
    let norms = embeddings
        .iter()
        .map(|embedding| norm(&embedding.vector))
        .collect::<Vec<_>>();

    for (left_index, left) in embeddings.iter().enumerate() {
        for (offset, right) in embeddings.iter().enumerate().skip(left_index + 1) {
            let Some(right_index) = passage_indices[offset] else {
                continue;
            };
            // Joining an already joined pair changes nothing, so its similarity is not needed.
            if union_find.find(left_index) == union_find.find(right_index) {
                continue;
            }
            let similarity = cosine(
                &left.vector,
                &right.vector,
                norms[left_index],
                norms[offset],
            );
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

fn norm(vector: &[f32]) -> f64 {
    vector
        .iter()
        .map(|value| f64::from(*value) * f64::from(*value))
        .sum::<f64>()
        .sqrt()
}

fn cosine(left: &[f32], right: &[f32], left_norm: f64, right_norm: f64) -> f64 {
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return 0.0;
    }
    let dot = left
        .iter()
        .zip(right)
        .map(|(left, right)| f64::from(*left) * f64::from(*right))
        .sum::<f64>();
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
