//! Access checks for explicit qualified paths through locally declared memberships.
//! Lexical and inherited lookup, library memberships and dot feature chains remain
//! separate services. Public aliases expose their target without changing its visibility.
use super::*;
use crate::language_frontend::lowering::collect::CollectedModule;

pub(super) struct QualifiedVisibility<'a> {
    context: &'a ResolverContext,
    aliases: &'a BTreeMap<String, QualifiedName>,
}
impl<'a> QualifiedVisibility<'a> {
    pub(super) fn new(context: &'a ResolverContext, aliases: &'a BTreeMap<String, QualifiedName>) -> Self {
        Self { context, aliases }
    }
    fn known(&self, name: &str) -> bool {
        self.context.local_definitions.contains_key(name)
            || self.context.local_usage_map.contains_key(name)
            || self.context.packages.iter().any(|p| p.qualified_name == name)
            || self.aliases.contains_key(name)
            || self.context.imports.iter().any(|import| !import.decl.is_expose
                && import.decl.path.segments.last().is_some_and(|member| {
                    !matches!(member.as_str(), "*" | "**")
                        && format!("{}.{}", import.owner_qualified_name.as_deref().unwrap_or("root"), member) == name
                }))
    }
    fn follow_alias(&self, mut name: String) -> Option<String> {
        let mut seen = BTreeSet::new();
        while let Some(target) = self.aliases.get(&name) {
            if !seen.insert(name) { return None; }
            name = target.as_dot_string();
        }
        Some(name)
    }
    pub(super) fn name(&self, name: &QualifiedName, scope: &str) -> Result<(), Diagnostic> {
        if name.segments.len() < 2 || name.span.start_line == 0 { return Ok(()); }
        let Some(mut current) = scoped_reference_candidates(&name.segments[0], scope)
            .into_iter().find(|candidate| self.known(candidate)) else { return Ok(()); };
        for segment in &name.segments[1..] {
            if matches!(segment.as_str(), "*" | "**") { break; }
            let Some(namespace) = self.follow_alias(current) else { return Ok(()); };
            current = format!("{namespace}.{segment}");
            // Check the named membership before following an alias to its element.
            if self.context.non_public_members.contains(&current) {
                return Err(Diagnostic::semantic(format!("qualified reference `{}` cannot access non-public membership `{current}`", name.as_colon_string()), Some(name.span.clone())));
            }
            if !self.known(&current) { break; }
        }
        Ok(())
    }
    pub(super) fn usage(&self, usage: &CollectedUsage) -> Result<(), Diagnostic> {
        let scope = &usage.owner_qualified_name;
        for name in usage.ty.iter().chain(&usage.additional_types)
            .chain(&usage.reference_target).chain(&usage.allocation_source).chain(&usage.allocation_target)
            .chain(&usage.specializes).chain(&usage.subsets).chain(&usage.redefines).chain(&usage.annotation_targets) {
            self.name(name, scope)?;
        }
        if let Some(expr) = &usage.expression { self.expression(expr, scope)?; }
        for member in &usage.members { self.usage(member)?; }
        Ok(())
    }
    fn expression(&self, expr: &Expr, scope: &str) -> Result<(), Diagnostic> {
        match expr {
            Expr::Name(name) | Expr::TypeReference(name) => self.name(name, scope)?,
            Expr::Operation { operands, .. } | Expr::Tuple { items: operands, .. } => {
                for value in operands { self.expression(value, scope)?; }
            }
            Expr::Unary { expr, .. } | Expr::NamedArgument { value: expr, .. }
                | Expr::Path { root: expr, .. } | Expr::Lambda { body: expr, .. } => self.expression(expr, scope)?,
            Expr::Binary { left, right, .. } => { self.expression(left, scope)?; self.expression(right, scope)?; }
            Expr::Call { args, .. } => { for value in args { self.expression(value, scope)?; } }
            Expr::Literal(_) | Expr::SelfRef(_) => {}
        }
        Ok(())
    }
    pub(super) fn module(&self, module: &CollectedModule) -> Result<(), Diagnostic> {
        for definition in &module.definitions {
            for name in &definition.specializes { self.name(name, &definition.qualified_name)?; }
            for usage in &definition.members { self.usage(usage)?; }
        }
        for usage in &module.usages { self.usage(usage)?; }
        for import in &module.imports { self.name(&import.decl.path, import.owner_qualified_name.as_deref().unwrap_or("root"))?; }
        for alias in &module.aliases {
            if let Some(decl) = &alias.declaration { self.name(&decl.target, &alias.owner_qualified_name)?; }
        }
        Ok(())
    }
}
