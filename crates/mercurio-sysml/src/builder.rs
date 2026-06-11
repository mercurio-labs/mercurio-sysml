use std::collections::BTreeMap;
use std::path::Path;

use mercurio_core::{
    AttributeWritePolicy, AuthoringError, AuthoringProject, ContainerSelector, KirDocument,
    Mutation, QualifiedName, SemanticEdit,
};
use serde_json::json;

use crate::{SysmlEnvironment, SysmlEnvironmentError, load_authoring_project_from_sysml};

pub mod stdlib {
    include!(concat!(
        env!("OUT_DIR"),
        "/stdlib_consts_sysml-2.0-metamodel-0.57.0.rs"
    ));
}

#[derive(Debug)]
pub enum BuilderError {
    Authoring(AuthoringError),
    Sysml(SysmlEnvironmentError),
    Io(std::io::Error),
}

impl std::fmt::Display for BuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Authoring(err) => write!(f, "{err}"),
            Self::Sysml(err) => write!(f, "{err}"),
            Self::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for BuilderError {}

impl From<AuthoringError> for BuilderError {
    fn from(value: AuthoringError) -> Self {
        Self::Authoring(value)
    }
}

impl From<SysmlEnvironmentError> for BuilderError {
    fn from(value: SysmlEnvironmentError) -> Self {
        Self::Sysml(value)
    }
}

impl From<std::io::Error> for BuilderError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub struct ModelBuilder {
    project: AuthoringProject,
    env: SysmlEnvironment,
    default_file: String,
    default_package: Option<QualifiedName>,
}

impl ModelBuilder {
    pub fn new() -> Result<Self, BuilderError> {
        Self::for_metamodel(crate::LATEST_SYSML_METAMODEL_ID)
    }

    pub fn for_metamodel(id: &str) -> Result<Self, BuilderError> {
        Ok(Self {
            project: load_authoring_project_from_sysml(BTreeMap::new())?,
            env: SysmlEnvironment::for_metamodel(id)?,
            default_file: "model.sysml".to_string(),
            default_package: None,
        })
    }

    pub fn in_package(mut self, name: impl Into<String>) -> Result<Self, BuilderError> {
        let package_name = QualifiedName::parse(&name.into());
        let result = self.project.apply_mutation(Mutation::AddPackage {
            target_file: self.default_file.clone(),
            package_name: package_name.clone(),
        })?;
        self.project.write_back_mutation(&result)?;
        for import in ["ISQ::*", "SI::*", "ScalarValues::*"] {
            let result = self.project.apply_mutation(Mutation::AddImport {
                target_file: self.default_file.clone(),
                package_name: Some(package_name.clone()),
                path: QualifiedName::parse(import),
            })?;
            self.project.write_back_mutation(&result)?;
        }
        self.default_package = Some(package_name);
        Ok(self)
    }

    pub fn add(&mut self, element: impl IntoDeclaration) -> Result<&mut Self, BuilderError> {
        let declaration = element.into_declaration();
        let container = self.default_container();
        self.apply_declaration(container, declaration)?;
        Ok(self)
    }

    pub fn to_sysml(&self) -> BTreeMap<String, String> {
        self.project
            .files()
            .filter_map(|(path, _)| {
                self.project
                    .render_new_file(path)
                    .ok()
                    .map(|source| (path.to_string(), source))
            })
            .collect()
    }

    pub fn compile(&self) -> Result<KirDocument, BuilderError> {
        let mut documents = Vec::new();
        for (path, source) in self.to_sysml() {
            documents.push(self.env.compile_text(&source, &path)?);
        }
        KirDocument::merge(documents)
            .map_err(AuthoringError::from)
            .map_err(Into::into)
    }

    pub fn save(&self, dir: &Path) -> Result<(), BuilderError> {
        for (path, source) in self.to_sysml() {
            let output = dir.join(path);
            if let Some(parent) = output.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(output, source)?;
        }
        Ok(())
    }

    fn default_container(&self) -> ContainerSelector {
        self.default_package
            .clone()
            .map(|qualified_name| ContainerSelector::Package { qualified_name })
            .unwrap_or_else(|| ContainerSelector::File {
                target_file: self.default_file.clone(),
            })
    }

    fn apply_declaration(
        &mut self,
        container: ContainerSelector,
        declaration: DeclarationSpec,
    ) -> Result<QualifiedName, BuilderError> {
        let name = declaration.name.clone();
        let mutation = match declaration.kind {
            DeclarationKind::Definition => Mutation::AddDefinition {
                container: container.clone(),
                keyword: declaration.keyword.clone(),
                name: name.clone(),
                specializes: declaration
                    .specializes
                    .iter()
                    .cloned()
                    .map(render_ref)
                    .collect(),
            },
            DeclarationKind::Usage => Mutation::AddUsage {
                container: container.clone(),
                keyword: declaration.keyword.clone(),
                name: name.clone(),
                ty: declaration.ty.clone().map(render_ref),
                specializes: declaration
                    .specializes
                    .iter()
                    .cloned()
                    .map(render_ref)
                    .collect(),
            },
        };
        let result = self.project.apply_mutation(mutation)?;
        self.project.write_back_mutation(&result)?;

        let qualified_name = child_name(&container, &name);
        if declaration.abstract_ {
            self.apply_semantic_attr(&qualified_name, "isAbstract", json!(true))?;
        }
        if let Some(expression) = declaration.expression {
            let result = self.project.apply_mutation(Mutation::SetExpression {
                qualified_name: qualified_name.clone(),
                expression: Some(expression),
            })?;
            self.project.write_back_mutation(&result)?;
        }
        if let Some(direction) = declaration.direction {
            self.apply_semantic_attr(&qualified_name, "direction", json!(direction))?;
        }
        if let Some(multiplicity) = declaration.multiplicity {
            self.apply_semantic_attr(&qualified_name, "multiplicity", json!(multiplicity))?;
        }

        let nested_container = ContainerSelector::Declaration {
            qualified_name: qualified_name.clone(),
        };
        for nested in declaration.nested {
            self.apply_declaration(nested_container.clone(), nested)?;
        }
        Ok(qualified_name)
    }

    fn apply_semantic_attr(
        &mut self,
        element: &QualifiedName,
        attribute: &str,
        value: serde_json::Value,
    ) -> Result<(), BuilderError> {
        let result = self
            .project
            .apply_semantic_edit(SemanticEdit::SetAttribute {
                element: element.clone(),
                attribute: attribute.to_string(),
                value,
                policy: AttributeWritePolicy::UpsertDirect,
            })?;
        self.project.write_back_mutation(&result)?;
        Ok(())
    }
}

fn child_name(container: &ContainerSelector, name: &str) -> QualifiedName {
    match container {
        ContainerSelector::File { .. } => QualifiedName::parse(name),
        ContainerSelector::Package { qualified_name }
        | ContainerSelector::Declaration { qualified_name } => {
            let mut segments = qualified_name.0.clone();
            segments.push(name.to_string());
            QualifiedName::new(segments)
        }
    }
}

fn render_ref(qualified_name: QualifiedName) -> QualifiedName {
    if qualified_name.0.len() > 1
        && matches!(
            qualified_name.0.first().map(String::as_str),
            Some("ISQ" | "SI" | "ScalarValues")
        )
    {
        return QualifiedName::new(vec![
            qualified_name.0.last().expect("checked non-empty").clone(),
        ]);
    }
    qualified_name
}

#[derive(Debug, Clone)]
pub struct StdlibRef(pub(crate) QualifiedName);

impl StdlibRef {
    pub fn qualified_name(&self) -> &QualifiedName {
        &self.0
    }
}

pub trait IntoRef {
    fn into_ref(self) -> QualifiedName;
}

impl IntoRef for StdlibRef {
    fn into_ref(self) -> QualifiedName {
        self.0
    }
}

impl IntoRef for &StdlibRef {
    fn into_ref(self) -> QualifiedName {
        self.0.clone()
    }
}

impl IntoRef for &str {
    fn into_ref(self) -> QualifiedName {
        QualifiedName::parse(self)
    }
}

impl IntoRef for String {
    fn into_ref(self) -> QualifiedName {
        QualifiedName::parse(&self)
    }
}

impl IntoRef for QualifiedName {
    fn into_ref(self) -> QualifiedName {
        self
    }
}

pub trait IntoDeclaration {
    fn into_declaration(self) -> DeclarationSpec;
}

#[derive(Debug, Clone, Copy)]
enum DeclarationKind {
    Definition,
    Usage,
}

#[derive(Debug, Clone)]
pub struct DeclarationSpec {
    kind: DeclarationKind,
    keyword: String,
    name: String,
    ty: Option<QualifiedName>,
    specializes: Vec<QualifiedName>,
    nested: Vec<DeclarationSpec>,
    expression: Option<String>,
    direction: Option<String>,
    multiplicity: Option<String>,
    abstract_: bool,
    docs: Vec<String>,
    ends: Vec<QualifiedName>,
}

impl DeclarationSpec {
    fn definition(keyword: &str, name: impl Into<String>) -> Self {
        Self::new(DeclarationKind::Definition, keyword, name)
    }

    fn usage(keyword: &str, name: impl Into<String>) -> Self {
        Self::new(DeclarationKind::Usage, keyword, name)
    }

    fn new(kind: DeclarationKind, keyword: &str, name: impl Into<String>) -> Self {
        Self {
            kind,
            keyword: keyword.to_string(),
            name: name.into(),
            ty: None,
            specializes: Vec::new(),
            nested: Vec::new(),
            expression: None,
            direction: None,
            multiplicity: None,
            abstract_: false,
            docs: Vec::new(),
            ends: Vec::new(),
        }
    }
}

macro_rules! definition_type {
    ($name:ident, $keyword:literal, [$($nested_method:ident : $nested_ty:ty),* $(,)?]) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            spec: DeclarationSpec,
        }

        impl $name {
            pub fn new(name: impl Into<String>) -> Self {
                Self { spec: DeclarationSpec::definition($keyword, name) }
            }

            pub fn specializes(mut self, target: impl IntoRef) -> Self {
                self.spec.specializes.push(target.into_ref());
                self
            }

            pub fn doc(mut self, text: &str) -> Self {
                self.spec.docs.push(text.to_string());
                self
            }

            pub fn abstract_(mut self) -> Self {
                self.spec.abstract_ = true;
                self
            }

            $(
                pub fn $nested_method(mut self, member: $nested_ty) -> Self {
                    self.spec.nested.push(member.into_declaration());
                    self
                }
            )*
        }

        impl IntoDeclaration for $name {
            fn into_declaration(self) -> DeclarationSpec {
                self.spec
            }
        }
    };
}

macro_rules! usage_type {
    ($name:ident, $keyword:literal) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            spec: DeclarationSpec,
        }

        impl $name {
            pub fn new(name: impl Into<String>) -> Self {
                Self {
                    spec: DeclarationSpec::usage($keyword, name),
                }
            }

            pub fn typed(mut self, target: impl IntoRef) -> Self {
                self.spec.ty = Some(target.into_ref());
                self
            }

            pub fn specializes(mut self, target: impl IntoRef) -> Self {
                self.spec.specializes.push(target.into_ref());
                self
            }

            pub fn doc(mut self, text: &str) -> Self {
                self.spec.docs.push(text.to_string());
                self
            }

            pub fn abstract_(mut self) -> Self {
                self.spec.abstract_ = true;
                self
            }

            pub fn multiplicity(mut self, raw: impl Into<String>) -> Self {
                self.spec.multiplicity = Some(raw.into());
                self
            }

            pub fn expression(mut self, raw: impl Into<String>) -> Self {
                self.spec.expression = Some(raw.into());
                self
            }

            pub fn direction(mut self, raw: impl Into<String>) -> Self {
                self.spec.direction = Some(raw.into());
                self
            }

            pub fn end(mut self, target: impl IntoRef) -> Self {
                self.spec.ends.push(target.into_ref());
                self
            }
        }

        impl IntoDeclaration for $name {
            fn into_declaration(self) -> DeclarationSpec {
                self.spec
            }
        }
    };
}

definition_type!(PartDefinition, "part", [
    with_part: PartUsage,
    with_attr: AttributeUsage,
    with_port: PortUsage,
]);
definition_type!(ItemDefinition, "item", [
    with_item: ItemUsage,
    with_attr: AttributeUsage,
]);
definition_type!(AttributeDefinition, "attribute", [
    with_attr: AttributeUsage,
]);
definition_type!(PortDefinition, "port", [
    with_attr: AttributeUsage,
]);
definition_type!(ConnectionDefinition, "connection", [
    with_end: ConnectionUsage,
]);
definition_type!(ActionDefinition, "action", [
    with_action: ActionUsage,
    with_attr: AttributeUsage,
]);
definition_type!(StateDefinition, "state", [
    with_state: StateUsage,
]);
definition_type!(RequirementDefinition, "requirement", [
    with_attr: AttributeUsage,
    with_part: PartUsage,
]);
definition_type!(InterfaceDefinition, "interface", [
    with_port: PortUsage,
]);

usage_type!(PartUsage, "part");
usage_type!(ItemUsage, "item");
usage_type!(AttributeUsage, "attribute");
usage_type!(PortUsage, "port");
usage_type!(ConnectionUsage, "connection");
usage_type!(ActionUsage, "action");
usage_type!(StateUsage, "state");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_and_compiles_part_hierarchy() {
        let mut model = ModelBuilder::new().unwrap().in_package("Demo").unwrap();

        model
            .add(
                PartDefinition::new("Engine")
                    .with_attr(AttributeUsage::new("mass").typed("ScalarValues::Real"))
                    .with_attr(AttributeUsage::new("power").typed("ScalarValues::Real")),
            )
            .unwrap();

        model
            .add(PartDefinition::new("Vehicle").with_part(PartUsage::new("engine").typed("Engine")))
            .unwrap();

        let sysml = model.to_sysml();
        assert!(sysml["model.sysml"].contains("part def Engine"));

        let kir = model.compile().unwrap();
        assert!(kir.elements.iter().any(|element| {
            element.id == "type.Demo.Engine"
                || element.properties.get("metatype")
                    == Some(&serde_json::json!("SysML::PartDefinition"))
        }));
    }
}
