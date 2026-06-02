//! Shared name and namespace helpers for lowering resolution.

use std::collections::BTreeMap;

use mercurio_language_contracts::ast::{QualifiedName, SourceSpan};

use crate::lowering::collect::ImportAliases;
use crate::lowering::ir::ResolvedPackage;
pub(crate) fn qualified_names_match(left: &QualifiedName, right: &QualifiedName) -> bool {
    left.segments == right.segments
        || qualified_name_suffix_matches(&left.segments, &right.segments)
        || qualified_name_suffix_matches(&right.segments, &left.segments)
}

pub(crate) fn qualified_name_suffix_matches(longer: &[String], shorter: &[String]) -> bool {
    longer.len() >= shorter.len() && longer[longer.len() - shorter.len()..] == *shorter
}

pub(crate) fn expand_import_namespace_prefix(
    name: &QualifiedName,
    local_aliases: &BTreeMap<String, QualifiedName>,
    import_aliases: &ImportAliases,
) -> Option<QualifiedName> {
    let first = name.segments.first()?;
    let prefix = import_aliases
        .namespace_aliases
        .get(first)
        .or_else(|| local_aliases.get(first))?;
    let mut segments = prefix.segments.clone();
    segments.extend(name.segments.iter().skip(1).cloned());
    let expanded = QualifiedName {
        segments,
        span: name.span.clone(),
    };
    (expanded != *name).then_some(expanded)
}

pub(crate) fn import_namespace_prefix(target_id: &str) -> String {
    target_id
        .split("::*")
        .next()
        .unwrap_or(target_id)
        .trim_end_matches("::")
        .to_string()
}

pub(crate) fn resolve_local_namespace_dot(
    namespace: &str,
    owner_package_qualified_name: &str,
    root_package: &Option<String>,
    packages: &[ResolvedPackage],
) -> Option<String> {
    let dotted = namespace.replace("::", ".");
    let mut candidates = vec![dotted.clone()];
    let mut cursor = owner_package_qualified_name;
    while !cursor.is_empty() {
        let candidate = format!("{cursor}.{dotted}");
        if !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
        let Some((parent, _)) = cursor.rsplit_once('.') else {
            break;
        };
        cursor = parent;
    }
    if let Some(root) = root_package {
        if dotted != *root && !dotted.starts_with(&format!("{root}.")) {
            candidates.push(format!("{root}.{dotted}"));
        }
    }

    for candidate in candidates {
        let matches = packages
            .iter()
            .filter(|package| {
                package.qualified_name == candidate
                    || package.qualified_name.ends_with(&format!(".{candidate}"))
            })
            .map(|package| package.qualified_name.clone())
            .collect::<Vec<_>>();
        if matches.len() == 1 {
            return matches.into_iter().next();
        }
    }

    None
}

pub(crate) fn direct_child_name<'a>(qualified_name: &'a str, prefix: &str) -> Option<&'a str> {
    let remainder = qualified_name.strip_prefix(prefix)?;
    if remainder.is_empty() || remainder.contains('.') || remainder.contains("::") {
        None
    } else {
        Some(remainder)
    }
}

pub(crate) fn dotted_name_to_qualified_name(value: &str, span: &SourceSpan) -> QualifiedName {
    QualifiedName {
        segments: value.split('.').map(str::to_string).collect(),
        span: span.clone(),
    }
}
