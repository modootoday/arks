//! Resolved ref target (direct OID or symbolic).

/// One hop of ref resolution: object id or `ref: …` target (not followed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefTarget {
    Direct(arks_oid::Oid),
    Symbolic(String),
}
