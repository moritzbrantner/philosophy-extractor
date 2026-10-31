use crate::model::{ClaimKind, ClaimRole, RelationKind};

pub fn claim_kind_from_label(label: &str) -> ClaimKind {
    match normalize_label(label).as_str() {
        "atomic" => ClaimKind::Atomic,
        "conditional" => ClaimKind::Conditional,
        "definition" => ClaimKind::Definition,
        "modal" => ClaimKind::Modal,
        "normative" => ClaimKind::Normative,
        "universal" => ClaimKind::Universal,
        _ => ClaimKind::Unknown,
    }
}

pub fn claim_role_from_label(label: &str) -> ClaimRole {
    match normalize_label(label).as_str() {
        "main_claim" | "mainclaim" => ClaimRole::MainClaim,
        "premise" => ClaimRole::Premise,
        "conclusion" => ClaimRole::Conclusion,
        "definition" => ClaimRole::Definition,
        "objection" => ClaimRole::Objection,
        "reply" => ClaimRole::Reply,
        "example" => ClaimRole::Example,
        "background" => ClaimRole::Background,
        "qualification" => ClaimRole::Qualification,
        _ => ClaimRole::Unknown,
    }
}

pub fn relation_kind_from_label(label: &str) -> Option<RelationKind> {
    match normalize_label(label).as_str() {
        "supports" => Some(RelationKind::Supports),
        "contradicts" => Some(RelationKind::Contradicts),
        "generalizes" => Some(RelationKind::Generalizes),
        "specializes" => Some(RelationKind::Specializes),
        "variant_of" | "variantof" => Some(RelationKind::VariantOf),
        _ => None,
    }
}

fn normalize_label(label: &str) -> String {
    label.trim().to_ascii_lowercase().replace(['-', ' '], "_")
}
