//! Lexical member lookup shared by feature references and inherited chains.
//! Traversal keys include both namespace and sought member; cycles terminate
//! without discarding an independently reachable specialization branch.
use super::*;

pub(super) struct FeatureLookup<'a> {
    pub stdlib_ids: &'a [String],
    pub stdlib_feature_index: &'a BTreeMap<String, BTreeMap<String, String>>,
    pub stdlib_aliases: &'a BTreeMap<String, String>,
    pub local_definitions: &'a BTreeMap<String, String>,
    pub local_aliases: &'a BTreeMap<String, QualifiedName>,
    pub import_aliases: &'a ImportAliases,
    pub definition_index: &'a BTreeMap<String, CollectedDefinition>,
    pub local_feature_index: &'a BTreeMap<String, BTreeMap<String, String>>,
    pub local_usage_map: &'a BTreeMap<String, CollectedUsage>,
}

#[cfg(test)]
mod library_visibility_tests {
    use super::*;

    #[test]
    fn inherited_library_feature_uses_declaring_membership_visibility() {
        let ids = vec!["Base::Private".to_string(), "Base::Protected".to_string(), "Base::Public".to_string()];
        let features = BTreeMap::from([("Derived".to_string(), BTreeMap::from([
            ("private".to_string(), "Base::Private".to_string()),
            ("protected".to_string(), "Base::Protected".to_string()),
            ("public".to_string(), "Base::Public".to_string()),
        ]))]);
        let imports = ImportAliases {
            library_membership_visibility: std::sync::Arc::new(BTreeMap::from([
                ("Base.Private".to_string(), "private".to_string()),
                ("Base.Protected".to_string(), "protected".to_string()),
                ("Base.Public".to_string(), "public".to_string()),
            ])),
            ..Default::default()
        };
        let empty = BTreeMap::new();
        let empty_aliases: BTreeMap<String, QualifiedName> = BTreeMap::new();
        let empty_definitions: BTreeMap<String, CollectedDefinition> = BTreeMap::new();
        let empty_features: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        let empty_usages: BTreeMap<String, CollectedUsage> = BTreeMap::new();
        let lookup = FeatureLookup {
            stdlib_ids: &ids, stdlib_feature_index: &features, stdlib_aliases: &empty,
            local_definitions: &empty, local_aliases: &empty_aliases, import_aliases: &imports,
            definition_index: &empty_definitions, local_feature_index: &empty_features, local_usage_map: &empty_usages,
        };
        for (name, lexical, inherited, visible) in [
            ("private", true, false, false),
            ("protected", true, true, false),
            ("public", true, true, true),
        ] {
            for (access, expected) in [
                (MemberAccess::Lexical, lexical),
                (MemberAccess::Inherited, inherited),
                (MemberAccess::Visible, visible),
            ] {
                assert_eq!(lookup.member_with_access("Derived", name, "", &mut BTreeSet::new(), access).is_some(), expected,
                    "{name} with {access:?}");
            }
        }
    }
}

/// KerML 8.2.3.5 and 7.3.2.3 distinguish membership access from target visibility.
#[derive(Clone, Copy, Debug)]
enum MemberAccess { Lexical, Inherited, Visible }
impl MemberAccess {
    fn permits(self, visibility: &str) -> bool {
        match self {
            Self::Lexical => true,
            Self::Inherited => visibility != "private",
            Self::Visible => visibility == "public",
        }
    }
    fn inherited(self) -> Self {
        match self { Self::Visible => Self::Visible, _ => Self::Inherited }
    }
}

impl FeatureLookup<'_> {
    pub(super) fn expression_target(
        &self,
        expression: &Expr,
        scope: &str,
        excluded: &str,
        seen: &mut BTreeSet<(String, String)>,
    ) -> Option<String> {
        match expression {
            Expr::TypeReference(name) => self.resolve(name, scope, excluded, seen),
            Expr::Operation {
                operator, operands, ..
            } if matches!(operator.as_str(), "as" | "meta") => operands
                .get(1)
                .and_then(|reference| self.expression_target(reference, scope, excluded, seen)),
            Expr::Operation {
                operator, operands, ..
            } if matches!(operator.as_str(), "#" | ".?" | "all") => operands
                .first()
                .and_then(|value| self.expression_target(value, scope, excluded, seen)),
            Expr::Name(name) => self.resolve(name, scope, excluded, seen),
            Expr::Path { root, segment, .. } => {
                let target = self.expression_target(root, scope, excluded, seen)?;
                self.member(&target, segment, excluded, seen)
            }
            Expr::Call { function, span, .. } => {
                let constructor = function.strip_prefix("new ");
                let name = QualifiedName {
                    segments: constructor
                        .unwrap_or(function)
                        .replace("::", ".")
                        .split('.')
                        .map(str::to_string)
                        .collect(),
                    span: span.clone(),
                };
                let target = resolve_type_reference_in_scope(
                    &name,
                    scope,
                    self.stdlib_ids,
                    self.stdlib_aliases,
                    self.local_definitions,
                    self.local_aliases,
                    self.import_aliases,
                )?;
                if constructor.is_some() {
                    return Some(target);
                }
                let definition = self.definition_index.get(target.strip_prefix("type.")?)?;
                if definition.construct == "Behavior" {
                    return Some(target);
                }
                let result = definition.members.iter().find(|m| {
                    m.modifiers.iter().any(|modifier| modifier == "return")
                        || m.construct == "ReturnUsage"
                })?;
                Some(collected_usage_element_id(result))
            }
            _ => None,
        }
    }
    pub(super) fn resolve(
        &self,
        name: &QualifiedName,
        scope: &str,
        excluded: &str,
        seen: &mut BTreeSet<(String, String)>,
    ) -> Option<String> {
        let key = (format!("resolve:{scope}"), name.as_colon_string());
        if !seen.insert(key.clone()) {
            return None;
        }
        let result = self.resolve_inner(name, scope, excluded, seen);
        seen.remove(&key);
        result
    }

    fn resolve_inner(
        &self,
        name: &QualifiedName,
        scope: &str,
        excluded: &str,
        seen: &mut BTreeSet<(String, String)>,
    ) -> Option<String> {
        let first = name.segments.first()?;
        let mut cursor = Some(scope);
        while let Some(owner) = cursor {
            if let Some(mut target) = self.member(owner, first, excluded, seen) {
                for segment in &name.segments[1..] {
                    target = self.member_with_access(&target, segment, excluded, seen, MemberAccess::Visible)?;
                }
                return Some(target);
            }
            let imported = format!("{owner}.{first}");
            if let Some(target) = self.import_aliases.value_aliases.get(&imported) {
                if let Some(result) =
                    self.chain(target.clone(), &name.segments[1..], excluded, seen)
                {
                    return Some(result);
                }
            }
            cursor = owner.rsplit_once('.').map(|(parent, _)| parent);
        }
        if !has_local_reference_prefix(name, self.local_definitions, self.local_feature_index,
            self.local_usage_map, self.local_aliases) {
            if let Some(resolved) = self.import_aliases.library_namespace_scope.resolve(
                &name.segments, self.stdlib_aliases,
            ) {
                return resolved;
            }
        }
        // Resolve a namespace/type prefix, then walk its inherited members.
        // Longest prefix wins; a dotted feature chain never becomes a loose
        // last-segment match against an unrelated namespace.
        for prefix_len in (1..=name.segments.len()).rev() {
            let prefix = QualifiedName {
                segments: name.segments[..prefix_len].to_vec(),
                span: name.span.clone(),
            };
            let dotted = prefix.as_dot_string();
            let mut prefixes = Vec::new();
            let mut cursor = Some(scope);
            while let Some(owner) = cursor {
                prefixes.push(format!("{owner}.{dotted}"));
                cursor = owner.rsplit_once('.').map(|(parent, _)| parent);
            }
            prefixes.push(dotted.clone());
            let target = prefixes
                .iter()
                .find_map(|p| self.local_usage_map.get(p).map(collected_usage_element_id))
                .or_else(|| {
                    resolve_type_reference_in_scope(
                        &prefix,
                        scope,
                        self.stdlib_ids,
                        self.stdlib_aliases,
                        self.local_definitions,
                        self.local_aliases,
                        self.import_aliases,
                    )
                });
            if let Some(target) = target {
                // A complete library ID can be found by the legacy type index
                // without walking its owning membership. Preserve explicit
                // aliases, whose visibility belongs to the alias membership.
                if self.library_membership_visibility(&target)
                        .is_some_and(|visibility| visibility != "public")
                    && !self.stdlib_aliases.get(&prefix.as_colon_string()).is_some_and(|id| id == &target)
                    && !self.local_aliases.contains_key(&prefix.as_dot_string())
                    && !self.import_aliases.value_aliases.contains_key(&prefix.as_dot_string())
                {
                    continue;
                }
                if let Some(result) =
                    self.chain(target, &name.segments[prefix_len..], excluded, seen)
                {
                    return Some(result);
                }
            }
        }
        None
    }

    fn library_membership_visibility(&self, target: &str) -> Option<&str> {
        let (owner, name) = target.rsplit_once("::")?;
        self.import_aliases.library_membership_visibility
            .get(&format!("{owner}.{name}"))
            .map(String::as_str)
    }

    fn chain(
        &self,
        mut target: String,
        segments: &[String],
        excluded: &str,
        seen: &mut BTreeSet<(String, String)>,
    ) -> Option<String> {
        for segment in segments {
            target = self.member_with_access(&target, segment, excluded, seen, MemberAccess::Visible)?;
        }
        (target != excluded).then_some(target)
    }

    pub(super) fn member(
        &self,
        owner: &str,
        name: &str,
        excluded: &str,
        seen: &mut BTreeSet<(String, String)>,
    ) -> Option<String> {
        self.member_with_access(owner, name, excluded, seen, MemberAccess::Lexical)
    }

    fn member_with_access(
        &self, owner: &str, name: &str, excluded: &str,
        seen: &mut BTreeSet<(String, String)>, access: MemberAccess,
    ) -> Option<String> {
        let owner = owner
            .strip_prefix("feature.")
            .or_else(|| owner.strip_prefix("type."))
            .unwrap_or(owner);
        let key = (format!("member:{access:?}:{owner}"), name.to_string());
        if !seen.insert(key.clone()) {
            return None;
        }
        let result = self.member_inner(owner, name, excluded, seen, access);
        seen.remove(&key);
        result
    }

    pub(super) fn type_specializes(
        &self,
        specific: &str,
        general: &str,
        library_parents: Option<&BTreeMap<String, Vec<String>>>,
        seen: &mut BTreeSet<String>,
    ) -> bool {
        if specific == general {
            return true;
        }
        if !seen.insert(specific.to_string()) {
            return false;
        }
        if let Some(definition) = specific
            .strip_prefix("type.")
            .and_then(|name| self.definition_index.get(name))
        {
            if definition
                .implicit_specializations
                .iter()
                .any(|parent| self.type_specializes(parent, general, library_parents, seen))
            {
                return true;
            }
            return definition.specializes.iter().any(|parent| {
                resolve_type_reference_in_scope(
                    parent,
                    &definition.qualified_name,
                    self.stdlib_ids,
                    self.stdlib_aliases,
                    self.local_definitions,
                    self.local_aliases,
                    self.import_aliases,
                )
                .is_some_and(|parent| {
                    self.type_specializes(&parent, general, library_parents, seen)
                })
            });
        }
        library_parents
            .and_then(|parents| parents.get(specific))
            .is_some_and(|parents| {
                parents
                    .iter()
                    .any(|parent| self.type_specializes(parent, general, library_parents, seen))
            })
    }

    fn member_inner(
        &self,
        owner: &str,
        name: &str,
        excluded: &str,
        seen: &mut BTreeSet<(String, String)>,
        access: MemberAccess,
    ) -> Option<String> {
        let visible = self.import_aliases.membership_visibility.get(&format!("{owner}.{name}"))
            .is_none_or(|visibility| access.permits(visibility));
        if visible {
        if let Some(target) = self.local_definitions.get(&format!("{owner}.{name}")) {
            return Some(target.clone());
        }
        if let Some(target) = self
            .local_feature_index
            .get(owner)
            .and_then(|m| m.get(name))
        {
            let id = feature_id_from_qualified_name(target);
            if id != excluded {
                return Some(id);
            }
        }
        // Aliases are memberships of their declaring namespace, including
        // aliases inherited through a usage's type. Never bind them globally.
        if let Some(target) = self.local_aliases.get(&format!("{owner}.{name}")) {
            if let Some(target) = self.resolve(target, owner, excluded, seen) { return Some(target); }
        }
        }
        if let Some(target) = self
            .stdlib_feature_index
            .get(owner)
            .and_then(|m| m.get(name))
        {
            let member_key = target.rsplit_once("::")
                .map(|(declaring_owner, declared_name)| format!("{declaring_owner}.{declared_name}"));
            let visible = member_key.as_deref()
                .and_then(|key| self.import_aliases.library_membership_visibility.get(key))
                .is_none_or(|visibility| access.permits(visibility));
            if visible && target != excluded {
                return Some(target.clone());
            }
        }
        // Conjugated port definitions inherit the original port's members.
        // Recognize only companions created by build_local_definition_map.
        if let Some((original, conjugated)) = owner.rsplit_once('.') {
            if let Some(definition) = self.definition_index.get(original) {
                if definition.construct == "PortDefinition"
                    && conjugated == format!("~{}", definition.declared_name)
                {
                    if let Some(member) = self.member_with_access(original, name, excluded, seen, access.inherited()) {
                        return Some(member);
                    }
                }
            }
        }
        if let Some(definition) = self.definition_index.get(owner) {
            let parents = definition
                .specializes
                .iter()
                .filter_map(|parent| {
                    resolve_type_reference_in_scope(
                        parent,
                        &definition.qualified_name,
                        self.stdlib_ids,
                        self.stdlib_aliases,
                        self.local_definitions,
                        self.local_aliases,
                        self.import_aliases,
                    ).or_else(|| {
                        self.resolve(parent, &definition.qualified_name, "", seen)
                            .filter(|target| target.starts_with("type."))
                    })
                })
                .collect::<Vec<_>>();
            for parent in &parents {
                // A directly listed general type must not mask a redefinition
                // inherited through another, more specific listed type.
                if parents.iter().any(|other| {
                    other != parent
                        && self.type_specializes(other, parent, None, &mut BTreeSet::new())
                }) {
                    continue;
                }
                if let Some(member) = self.member_with_access(parent, name, excluded, seen, access.inherited()) {
                    return Some(member);
                }
            }
            for parent in &definition.implicit_specializations {
                if let Some(member) = self.member_with_access(parent, name, excluded, seen, access.inherited()) {
                    return Some(member);
                }
            }
        }
        if let Some(usage) = self.local_usage_map.get(owner) {
            // The accepted payload is visible from the transition namespace,
            // while its owning membership remains on the accepter action.
            if usage.construct == "TransitionUsage" {
                if let Some(member) =
                    self.member(&format!("{owner}.accepter"), name, excluded, seen)
                {
                    return Some(member);
                }
            }
            let own_id = feature_id_from_qualified_name(owner);
            for parent in usage
                .specializes
                .iter()
                .chain(&usage.subsets)
                .chain(&usage.redefines)
                .chain(usage.reference_target.iter())
            {
                if let Some(id) = self.resolve(parent, &usage.owner_qualified_name, &own_id, seen) {
                    if let Some(member) = self.member_with_access(&id, name, excluded, seen, access.inherited()) {
                        return Some(member);
                    }
                }
            }
            for ty in usage.ty.iter().chain(usage.additional_types.iter()) {
                if let Some(id) = resolve_type_reference_in_scope(
                    ty,
                    &usage.owner_qualified_name,
                    self.stdlib_ids,
                    self.stdlib_aliases,
                    self.local_definitions,
                    self.local_aliases,
                    self.import_aliases,
                )
                .or_else(|| {
                    self.resolve(
                        ty,
                        &usage.owner_qualified_name,
                        &feature_id_from_qualified_name(owner),
                        seen,
                    )
                    .filter(|target| target.starts_with("type."))
                }) {
                    if let Some(member) = self.member_with_access(&id, name, excluded, seen, access.inherited()) {
                        return Some(member);
                    }
                }
            }
            let own_id = feature_id_from_qualified_name(owner);
            if usage
                .modifiers
                .iter()
                .any(|modifier| matches!(modifier.as_str(), "in" | "out" | "inout" | "return"))
            {
                if let Some(inherited) = self.member(
                    &usage.owner_qualified_name,
                    &usage.declared_name,
                    &own_id,
                    seen,
                ) {
                    if let Some(member) = self.member_with_access(&inherited, name, excluded, seen, access.inherited()) {
                        return Some(member);
                    }
                }
            }
            if let Some(expression) = &usage.expression {
                if let Some(target) =
                    self.expression_target(expression, &usage.owner_qualified_name, &own_id, seen)
                {
                    if let Some(member) = self.member_with_access(&target, name, excluded, seen, access.inherited()) {
                        return Some(member);
                    }
                }
            }
            for parent in &usage.implicit_subsets {
                if let Some(member) = self.member_with_access(parent, name, excluded, seen, access.inherited()) {
                    return Some(member);
                }
            }
            if let Some(implicit_type) = &usage.implicit_type {
                if let Some(member) = self.member_with_access(implicit_type, name, excluded, seen, access.inherited()) {
                    return Some(member);
                }
            }
        }
        None
    }
}
