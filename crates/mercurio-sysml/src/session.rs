use std::collections::BTreeMap;

use mercurio_core::{ForkElement, ForkElementSpec, ModelFork, SessionError};
use serde_json::Value;

pub const SYSML_REQUIREMENT_USAGE_KIND: &str = "Model::RequirementUsage";
pub const SYSML_PART_USAGE_KIND: &str = "Model::PartUsage";
pub const SYSML_SATISFY_KEYWORD: &str = "satisfy";
pub const SYSML_VERIFY_KEYWORD: &str = "verify";

pub trait SysmlModelForkExt {
    fn sysml_requirement(
        &mut self,
        owner: &ForkElement,
        name: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<ForkElement, SessionError>;

    fn sysml_part(
        &mut self,
        owner: &ForkElement,
        name: impl Into<String>,
        ty: Option<impl Into<String>>,
    ) -> Result<ForkElement, SessionError>;

    fn sysml_satisfy(
        &mut self,
        owner: &ForkElement,
        target: &ForkElement,
    ) -> Result<ForkElement, SessionError>;

    fn sysml_verify(
        &mut self,
        owner: &ForkElement,
        target: &ForkElement,
    ) -> Result<ForkElement, SessionError>;
}

impl SysmlModelForkExt for ModelFork {
    fn sysml_requirement(
        &mut self,
        owner: &ForkElement,
        name: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<ForkElement, SessionError> {
        self.semantic_element(
            owner,
            ForkElementSpec {
                id_prefix: "requirement".to_string(),
                kind: SYSML_REQUIREMENT_USAGE_KIND.to_string(),
                name: name.into(),
                properties: BTreeMap::from([("doc".to_string(), doc_metadata(text.into()))]),
            },
        )
    }

    fn sysml_part(
        &mut self,
        owner: &ForkElement,
        name: impl Into<String>,
        ty: Option<impl Into<String>>,
    ) -> Result<ForkElement, SessionError> {
        let mut properties = BTreeMap::new();
        if let Some(ty) = ty {
            properties.insert("type".to_string(), Value::String(ty.into()));
        }
        self.semantic_element(
            owner,
            ForkElementSpec {
                id_prefix: "feature".to_string(),
                kind: SYSML_PART_USAGE_KIND.to_string(),
                name: name.into(),
                properties,
            },
        )
    }

    fn sysml_satisfy(
        &mut self,
        owner: &ForkElement,
        target: &ForkElement,
    ) -> Result<ForkElement, SessionError> {
        self.relationship(owner, SYSML_SATISFY_KEYWORD, target)
    }

    fn sysml_verify(
        &mut self,
        owner: &ForkElement,
        target: &ForkElement,
    ) -> Result<ForkElement, SessionError> {
        self.relationship(owner, SYSML_VERIFY_KEYWORD, target)
    }
}

fn doc_metadata(text: String) -> Value {
    Value::Array(vec![Value::String(text)])
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use mercurio_core::{KirDocument, ModelWorkspace, WorkspaceSnapshot};

    use super::*;

    #[test]
    fn sysml_fork_extension_adds_requirement_part_and_trace_relationship() {
        let workspace = ModelWorkspace::new(
            WorkspaceSnapshot::new(KirDocument {
                metadata: BTreeMap::new(),
                elements: Vec::new(),
            })
            .unwrap(),
        );
        let mut fork = workspace.session().fork("sysml extension");

        let package = fork.package("Demo", None).unwrap();
        let requirement = fork
            .sysml_requirement(&package, "SafeStart", "Vehicle starts safely")
            .unwrap();
        let controller = fork
            .sysml_part(&package, "controller", Option::<String>::None)
            .unwrap();
        let satisfy = fork.sysml_satisfy(&controller, &requirement).unwrap();

        assert_eq!(requirement.id, "requirement.Demo.SafeStart");
        assert_eq!(controller.id, "feature.Demo.controller");
        assert!(
            satisfy
                .id
                .starts_with("relationship.Demo.controller.satisfy_")
        );
    }
}
