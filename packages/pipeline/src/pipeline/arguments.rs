use crate::model::{
    ArgumentCandidate, ArgumentRoleLink, ClaimCandidate, ClaimRole, PassageCluster,
    RelationCandidate, RelationKind,
};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

use super::digest_text;

pub fn reconstruct_arguments(
    clusters: &[PassageCluster],
    claims: &[ClaimCandidate],
    roles: &[crate::model::RoleAssignment],
) -> Vec<ArgumentCandidate> {
    let role_by_claim = roles
        .iter()
        .map(|role| (role.claim_id.as_str(), role.primary_role))
        .collect::<HashMap<_, _>>();
    let mut arguments = Vec::new();
    for cluster in clusters {
        let cluster_claims = claims
            .iter()
            .filter(|claim| {
                claim
                    .source_passage_ids
                    .iter()
                    .any(|id| cluster.passage_ids.contains(id))
            })
            .collect::<Vec<_>>();
        if cluster_claims.len() < 2 {
            continue;
        }
        let conclusion = cluster_claims
            .iter()
            .find(|claim| {
                matches!(
                    role_by_claim.get(claim.id.as_str()),
                    Some(ClaimRole::Conclusion | ClaimRole::MainClaim)
                )
            })
            .copied()
            .unwrap_or(cluster_claims[cluster_claims.len() - 1]);
        let premise_claim_ids = cluster_claims
            .iter()
            .filter(|claim| claim.id != conclusion.id)
            .map(|claim| claim.id.clone())
            .collect::<Vec<_>>();
        if premise_claim_ids.is_empty() {
            continue;
        }
        let support_relations = premise_claim_ids
            .iter()
            .map(|premise_id| ArgumentRoleLink {
                from_claim_id: premise_id.clone(),
                to_claim_id: conclusion.id.clone(),
                relation: RelationKind::Supports,
                confidence: 0.6,
            })
            .collect::<Vec<_>>();
        arguments.push(ArgumentCandidate {
            id: format!(
                "arg_{}",
                &digest_text(&format!("{}{}", cluster.id, conclusion.id))[..16]
            ),
            cluster_id: cluster.id.clone(),
            premise_claim_ids,
            conclusion_claim_id: conclusion.id.clone(),
            objection_claim_ids: Vec::new(),
            support_relations,
            attack_relations: Vec::new(),
            confidence: 0.58,
            rationale: Some(
                "Deterministic MVP argument reconstruction within passage cluster.".to_string(),
            ),
        });
    }
    arguments
}

pub fn argument_relations(arguments: &[ArgumentCandidate]) -> Vec<RelationCandidate> {
    let mut relations = Vec::new();
    for argument in arguments {
        for support in &argument.support_relations {
            let mut metadata = BTreeMap::new();
            metadata.insert("argumentId".to_string(), Value::String(argument.id.clone()));
            metadata.insert("reviewRequired".to_string(), Value::Bool(true));
            relations.push(RelationCandidate {
                from_proposition_id: support.from_claim_id.clone(),
                to_proposition_id: support.to_claim_id.clone(),
                kind: support.relation,
                confidence: support.confidence,
                note: Some("Argument reconstruction linked premise to conclusion.".to_string()),
                metadata,
            });
        }
    }
    relations
}
