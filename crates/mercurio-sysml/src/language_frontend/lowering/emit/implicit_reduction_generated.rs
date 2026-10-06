// Generated pinned reduction kind order; semantic traversal is handwritten.
pub(super) fn rank(kind: &str) -> Option<usize> { match kind {
    "ConjugatedPortTyping" => Some(31),
    "CrossSubsetting" => Some(41),
    "FeatureTyping" => Some(64),
    "Redefinition" => Some(126),
    "ReferenceSubsetting" => Some(127),
    "Specialization" => Some(141),
    "Subclassification" => Some(148),
    "Subsetting" => Some(150),
    _ => None,
} }
