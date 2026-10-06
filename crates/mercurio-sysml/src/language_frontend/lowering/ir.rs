use std::collections::BTreeMap;

use serde_json::Value;

use mercurio_foundation::language_contracts::ast::{
    BinaryOp, MultiplicityRange, SourceSpan, UnaryOp,
};

#[derive(Debug, Clone)]
pub struct ResolvedModule {
    pub aliases: Vec<ResolvedAlias>,
    pub packages: Vec<ResolvedPackage>,
    pub imports: Vec<ResolvedImport>,
    pub definitions: Vec<ResolvedDefinition>,
    pub usages: Vec<ResolvedUsage>,
}

#[derive(Debug, Clone)]
pub struct ResolvedAlias {
    pub qualified_name: String,
    pub owner_qualified_name: String,
    pub declared_name: String,
    pub declared_short_name: Option<String>,
    pub target: String,
    pub visibility: String,
    pub members: Vec<ResolvedUsage>,
    pub docs: Vec<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    pub owner_package_qualified_name: Option<String>,
    pub qualified_name: String,
    pub declared_name: String,
    pub modifiers: Vec<String>,
    pub docs: Vec<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct ResolvedImport {
    pub original_path: mercurio_foundation::language_contracts::ast::QualifiedName,
    pub alias_membership_id: Option<String>,
    pub referenced_element_id: Option<String>,
    pub members: Vec<ResolvedUsage>,
    /// Qualified name of the owning namespace — a package, a definition, or a
    /// *usage*. A `view v { expose x::**; }` records the view usage here, so
    /// emission must not assume this names a package (save-as-view SV-2).
    pub owner_qualified_name: Option<String>,
    pub target_id: String,
    pub imported_name: Option<String>,
    /// `true` for `expose`, which lowers to `SysML::Expose` rather than
    /// `SysML::Import` (save-as-view SV-1).
    pub is_expose: bool,
    /// Public imports contribute members to downstream namespace imports.
    pub is_public: bool,
    pub visibility: String,
    pub is_import_all: bool,
    /// Verbatim namespace-query filter condition, if any.
    pub filter: Option<String>,
    pub docs: Vec<String>,
    pub span: SourceSpan,
    pub ordinal: usize,
}

#[derive(Debug, Clone)]
pub struct ResolvedDefinition {
    pub visibility: String,
    pub construct: String,
    pub qualified_name: String,
    pub declared_name: String,
    pub declared_short_name: Option<String>,
    pub is_anonymous: bool,
    pub is_abstract: bool,
    pub is_variation: bool,
    pub specializes: Vec<String>,
    pub members: Vec<ResolvedUsage>,
    pub docs: Vec<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct ResolvedAnnotationTarget {
    pub target: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct ResolvedUsage {
    pub annotation_targets: Vec<ResolvedAnnotationTarget>,
    /// Derived language flags computed from resolved ownership and typing.
    pub derived_properties: BTreeMap<String, bool>,
    pub construct: String,
    pub owner_construct: String,
    pub owner_qualified_name: String,
    pub qualified_name: String,
    pub declared_name: String,
    pub is_implicit_name: bool,
    pub has_explicit_type: bool,
    /// Source-owned specializations, excluding derived library defaults.
    pub has_explicit_specialization: bool,
    pub type_ref: Option<String>,
    pub additional_type_refs: Vec<String>,
    pub reference_target: Option<String>,
    pub related_features: Vec<String>,
    pub allocation_source: Option<String>,
    pub allocation_target: Option<String>,
    pub metadata_properties: BTreeMap<String, String>,
    pub multiplicity: Option<MultiplicityRange>,
    pub expression: Option<ResolvedExpr>,
    pub is_derived: bool,
    pub specializes: Vec<String>,
    pub specialized_features: Vec<String>,
    pub subsetted_features: Vec<String>,
    pub redefined_features: Vec<String>,
    pub members: Vec<ResolvedUsage>,
    pub modifiers: Vec<String>,
    pub docs: Vec<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedExpr {
    Operation {
        operator: String,
        operands: Vec<ResolvedExpr>,
    },
    TypeReference {
        target: String,
    },
    Variable {
        name: String,
    },
    NamedArgument {
        parameter: String,
        value: Box<ResolvedExpr>,
    },
    Lambda {
        parameters: Vec<ResolvedExpressionParameter>,
        body: Box<ResolvedExpr>,
    },
    Literal(Value),
    SelfRef,
    Tuple {
        items: Vec<ResolvedExpr>,
    },
    FeaturePath {
        segments: Vec<ResolvedPathSegment>,
    },
    Select {
        root: Box<ResolvedExpr>,
        segments: Vec<ResolvedPathSegment>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<ResolvedExpr>,
    },
    Binary {
        left: Box<ResolvedExpr>,
        op: BinaryOp,
        right: Box<ResolvedExpr>,
    },
    Call {
        function: String,
        args: Vec<ResolvedExpr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPathSegment {
    pub name: String,
    pub feature_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedExpressionParameter {
    pub name: String,
    pub type_ref: Option<String>,
    pub properties: std::collections::BTreeMap<String, Value>,
    pub default: Option<Box<ResolvedExpr>>,
}
