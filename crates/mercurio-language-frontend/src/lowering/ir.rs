use std::collections::BTreeMap;

use serde_json::Value;

use mercurio_language_contracts::ast::{BinaryOp, MultiplicityRange, SourceSpan, UnaryOp};

#[derive(Debug, Clone)]
pub struct ResolvedModule {
    pub packages: Vec<ResolvedPackage>,
    pub imports: Vec<ResolvedImport>,
    pub definitions: Vec<ResolvedDefinition>,
    pub usages: Vec<ResolvedUsage>,
}

#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    pub owner_package_qualified_name: Option<String>,
    pub qualified_name: String,
    pub declared_name: String,
    pub docs: Vec<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct ResolvedImport {
    pub owner_package_qualified_name: Option<String>,
    pub target_id: String,
    pub imported_name: Option<String>,
    pub docs: Vec<String>,
    pub span: SourceSpan,
    pub ordinal: usize,
}

#[derive(Debug, Clone)]
pub struct ResolvedDefinition {
    pub construct: String,
    pub qualified_name: String,
    pub declared_name: String,
    pub is_abstract: bool,
    pub specializes: Vec<String>,
    pub members: Vec<ResolvedUsage>,
    pub docs: Vec<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct ResolvedUsage {
    pub construct: String,
    pub owner_construct: String,
    pub owner_qualified_name: String,
    pub qualified_name: String,
    pub declared_name: String,
    pub is_implicit_name: bool,
    pub has_explicit_type: bool,
    pub type_ref: Option<String>,
    pub additional_type_refs: Vec<String>,
    pub reference_target: Option<String>,
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
    Literal(Value),
    SelfRef,
    Tuple {
        items: Vec<ResolvedExpr>,
    },
    FeaturePath {
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
