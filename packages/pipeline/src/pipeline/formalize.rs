use crate::model::{
    ClaimRole, FormalEvaluation, FormalEvaluationStatus, FormalLogicAst, FormalizationCandidate,
    FormalizationStatus, NormalizedProposition, PipelineDiagnostic, PropositionCandidate,
};
use std::collections::{BTreeMap, HashMap};

use super::config::PipelineConfig;

pub fn normalized_propositions(
    candidates: &[PropositionCandidate],
    roles: &[crate::model::RoleAssignment],
) -> Vec<NormalizedProposition> {
    let role_by_claim = roles
        .iter()
        .map(|role| (role.claim_id.as_str(), role.primary_role))
        .collect::<HashMap<_, _>>();
    let mut by_text: BTreeMap<String, NormalizedProposition> = BTreeMap::new();
    for candidate in candidates {
        by_text
            .entry(candidate.canonical_text.clone())
            .and_modify(|existing| {
                existing.source_claim_ids.push(candidate.id.clone());
                for passage_id in &candidate.source_fragment_ids {
                    if !existing.source_passage_ids.contains(passage_id) {
                        existing.source_passage_ids.push(passage_id.clone());
                    }
                }
                for segment in &candidate.source_segments {
                    if !existing
                        .source_segments
                        .iter()
                        .any(|known| known.segment_id == segment.segment_id)
                    {
                        existing.source_segments.push(segment.clone());
                    }
                }
                existing.confidence = existing.confidence.max(candidate.confidence);
            })
            .or_insert_with(|| NormalizedProposition {
                id: candidate.id.clone(),
                canonical_text: candidate.canonical_text.clone(),
                source_claim_ids: vec![candidate.id.clone()],
                source_passage_ids: candidate.source_fragment_ids.clone(),
                source_segments: candidate.source_segments.clone(),
                claim_kind: candidate.claim_kind,
                role: role_by_claim
                    .get(candidate.id.as_str())
                    .copied()
                    .unwrap_or(ClaimRole::Unknown),
                confidence: candidate.confidence,
            });
    }
    by_text.into_values().collect()
}

pub fn formalize(propositions: &[NormalizedProposition]) -> Vec<FormalizationCandidate> {
    propositions
        .iter()
        .map(|proposition| {
            let predicate = sanitize_identifier(&proposition.id);
            let ast = if proposition
                .canonical_text
                .to_ascii_lowercase()
                .contains(" not ")
            {
                FormalLogicAst::Not {
                    value: Box::new(FormalLogicAst::Atomic {
                        predicate,
                        args: Vec::new(),
                    }),
                }
            } else {
                FormalLogicAst::Atomic {
                    predicate,
                    args: Vec::new(),
                }
            };
            let lean_expression = lean_expression_for_ast(&ast);
            FormalizationCandidate {
                id: format!("formal_{}", proposition.id),
                proposition_id: proposition.id.clone(),
                ast,
                lean_expression,
                status: FormalizationStatus::SchemaValid,
                confidence: 0.5,
                diagnostics: Vec::new(),
            }
        })
        .collect()
}

pub fn evaluate_formalizations(
    formalizations: &[FormalizationCandidate],
    config: &PipelineConfig,
) -> Vec<FormalEvaluation> {
    formalizations
        .iter()
        .map(|candidate| FormalEvaluation {
            formalization_id: candidate.id.clone(),
            proposition_id: candidate.proposition_id.clone(),
            status: FormalEvaluationStatus::Unsupported,
            lean_source: Some(format!(
                "-- Lean binary: {}\ndef {} : Prop := {}\n#check {}\n",
                config.lean_bin,
                sanitize_identifier(&candidate.id),
                candidate.lean_expression,
                sanitize_identifier(&candidate.id)
            )),
            diagnostics: vec![PipelineDiagnostic {
                code: "unsupported_goal".to_string(),
                severity: "info".to_string(),
                message: "Lean MVP generated typecheckable source; proof search is unsupported for this candidate.".to_string(),
                target_id: Some(candidate.id.clone()),
            }],
        })
        .collect()
}

fn sanitize_identifier(value: &str) -> String {
    let mut output = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            output.push(ch);
        } else {
            output.push('_');
        }
    }
    if output
        .chars()
        .next()
        .is_none_or(|ch| !ch.is_ascii_alphabetic())
    {
        output.insert(0, 'p');
    }
    output
}

fn lean_expression_for_ast(ast: &FormalLogicAst) -> String {
    match ast {
        FormalLogicAst::Atomic { .. } => "True".to_string(),
        FormalLogicAst::Not { value } => format!("Not ({})", lean_expression_for_ast(value)),
        FormalLogicAst::And { values } => values
            .iter()
            .map(lean_expression_for_ast)
            .collect::<Vec<_>>()
            .join(" /\\ "),
        FormalLogicAst::Or { values } => values
            .iter()
            .map(lean_expression_for_ast)
            .collect::<Vec<_>>()
            .join(" \\/ "),
        FormalLogicAst::Implies { left, right } => {
            format!(
                "({}) -> ({})",
                lean_expression_for_ast(left),
                lean_expression_for_ast(right)
            )
        }
        FormalLogicAst::Iff { left, right } => {
            format!(
                "Iff ({}) ({})",
                lean_expression_for_ast(left),
                lean_expression_for_ast(right)
            )
        }
        FormalLogicAst::ForAll { variable, body } => {
            format!(
                "forall {variable} : Prop, {}",
                lean_expression_for_ast(body)
            )
        }
        FormalLogicAst::Exists { variable, body } => {
            format!(
                "exists {variable} : Prop, {}",
                lean_expression_for_ast(body)
            )
        }
    }
}
