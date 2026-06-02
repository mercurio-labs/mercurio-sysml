use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use mercurio_core::{
    DerivedFeatureManifest, DerivedFeatureRegistry, DerivedFeatureRule, DerivedFeatureSpec,
    KIR_SCHEMA_VERSION, KirDocument, KirElement, builtin_core_derived_feature_manifest,
};
use serde_json::{Value, json};

pub const MERCURIO_WORKSPACE_ROOT_ENV: &str = "MERCURIO_WORKSPACE_ROOT";
pub const MERCURIO_PILOT_ROOT_ENV: &str = "MERCURIO_PILOT_ROOT";
pub const MERCURIO_EXAMPLES_ROOT_ENV: &str = "MERCURIO_EXAMPLES_ROOT";

const PILOT_REPO_NAME: &str = "SysML-v2-Pilot-Implementation";
const EXAMPLES_REPO_NAME: &str = "mercurio-examples";

pub fn default_pilot_root() -> PathBuf {
    if let Some(path) = env_path(MERCURIO_PILOT_ROOT_ENV) {
        return path;
    }

    if let Some(workspace_root) = env_path(MERCURIO_WORKSPACE_ROOT_ENV) {
        let sibling = workspace_root.join(PILOT_REPO_NAME);
        if sibling.exists() {
            return sibling;
        }
        return workspace_root.join("external").join(PILOT_REPO_NAME);
    }

    let external = PathBuf::from("../external").join(PILOT_REPO_NAME);
    if external.exists() {
        external
    } else {
        PathBuf::from("..").join(PILOT_REPO_NAME)
    }
}

pub fn default_kerml_examples_root(fallback_in_core: impl Into<PathBuf>) -> PathBuf {
    let fallback_in_core = fallback_in_core.into();

    if let Some(path) = env_path(MERCURIO_EXAMPLES_ROOT_ENV) {
        let kerml_examples = path.join("kerml").join("examples");
        if kerml_examples.exists() {
            return kerml_examples;
        }
        return path;
    }

    if let Some(workspace_root) = env_path(MERCURIO_WORKSPACE_ROOT_ENV) {
        let examples_root = workspace_root
            .join(EXAMPLES_REPO_NAME)
            .join("kerml")
            .join("examples");
        if examples_root.exists() {
            return examples_root;
        }
    }

    fallback_in_core
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

pub fn sha256_file(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut file = std::fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(sha256_hex(&bytes))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut state = Sha256::new();
    state.update(bytes);
    state
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub struct LanguageBaselineSplit {
    pub kernel: KirDocument,
    pub sysml_delta: KirDocument,
}

pub fn split_language_baselines(
    source: KirDocument,
    source_path: impl Into<String>,
) -> LanguageBaselineSplit {
    let source_path = source_path.into();
    let (kernel_elements, sysml_elements) = split_library_elements(source.elements);

    LanguageBaselineSplit {
        kernel: split_document(
            "org.omg/kerml-kernel",
            "KerML/Kernel baseline extracted from the bundled SysML pilot stdlib.",
            &source_path,
            kernel_elements,
        ),
        sysml_delta: split_document(
            "org.omg/sysml-library",
            "SysML library delta extracted from the bundled SysML pilot stdlib. KerML/Kernel elements are intentionally excluded.",
            &source_path,
            sysml_elements,
        ),
    }
}

pub fn attach_core_derived_feature_manifest(
    document: &mut KirDocument,
    metamodel: impl Into<String>,
) -> Result<(), serde_json::Error> {
    document.metadata.insert(
        "derived_feature_manifest".to_string(),
        serde_json::to_value(builtin_core_derived_feature_manifest(Some(
            metamodel.into(),
        )))?,
    );
    Ok(())
}

pub fn attach_stdlib_derived_feature_manifest(
    document: &mut KirDocument,
    metamodel: impl Into<String>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let manifest = stdlib_derived_feature_manifest(document, Some(metamodel.into()))?;
    let spec_count = manifest.derived_features.len();
    document.metadata.insert(
        "derived_feature_manifest".to_string(),
        serde_json::to_value(manifest)?,
    );
    Ok(spec_count)
}

pub fn stdlib_derived_feature_manifest(
    document: &KirDocument,
    metamodel: Option<String>,
) -> Result<DerivedFeatureManifest, Box<dyn std::error::Error>> {
    let mut manifest = builtin_core_derived_feature_manifest(metamodel);
    let mut generated_by_target = BTreeMap::new();
    for spec in document
        .elements
        .iter()
        .filter_map(derived_feature_subset_chain_spec)
    {
        let target = match &spec.rule {
            DerivedFeatureRule::SubsetChain { target_feature, .. } => target_feature.clone(),
            DerivedFeatureRule::IntersectionSubsetChain { target_feature, .. } => {
                target_feature.clone()
            }
            _ => continue,
        };
        generated_by_target.entry(target).or_insert(spec);
    }
    let mut generated = generated_by_target.into_values().collect::<Vec<_>>();
    generated.sort_by(|left, right| {
        left.owner
            .cmp(&right.owner)
            .then_with(|| left.feature.cmp(&right.feature))
    });
    manifest.derived_features.extend(generated);
    DerivedFeatureRegistry::from_manifest(manifest.clone()).validate()?;
    Ok(manifest)
}

fn derived_feature_subset_chain_spec(element: &KirElement) -> Option<DerivedFeatureSpec> {
    if !is_derived_feature(element) {
        return None;
    }

    let metadata = element
        .properties
        .get("metadata")
        .and_then(Value::as_object);
    let owner = string_property(element, metadata, "owner")?;
    let declared_name = string_property(element, metadata, "declared_name")
        .or_else(|| element.id.rsplit("::").next().map(str::to_string))?;
    if is_implemented_core_feature(&owner, &declared_name) {
        return None;
    }

    let type_refs = string_list_property(element, "type");
    let redefines = string_list_property(element, "redefines")
        .into_iter()
        .chain(string_list_property(element, "redefined_features"))
        .collect::<Vec<_>>();
    if !redefines.is_empty() {
        return None;
    }

    let specializes = string_list_property(element, "specializes");
    let candidate_sources = candidate_derivation_sources(&type_refs, &specializes, &[]);
    if candidate_sources.is_empty() {
        return None;
    }

    let target_feature =
        string_property(element, metadata, "source_feature").unwrap_or_else(|| element.id.clone());
    let rule = if candidate_sources.len() == 1 {
        DerivedFeatureRule::SubsetChain {
            source: candidate_sources.into_iter().next()?,
            target_feature,
            target_kind: None,
            target_type: type_refs.into_iter().next(),
        }
    } else {
        DerivedFeatureRule::IntersectionSubsetChain {
            sources: candidate_sources,
            target_feature,
            target_kind: None,
            target_type: type_refs.into_iter().next(),
        }
    };

    Some(DerivedFeatureSpec {
        owner,
        feature: declared_name,
        rule,
    })
}

fn is_derived_feature(element: &KirElement) -> bool {
    element
        .properties
        .get("metadata")
        .and_then(Value::as_object)
        .and_then(|metadata| metadata.get("is_derived"))
        .and_then(Value::as_bool)
        == Some(true)
        || element
            .properties
            .get("is_derived")
            .and_then(Value::as_bool)
            == Some(true)
}

fn candidate_derivation_sources(
    type_refs: &[String],
    specializes: &[String],
    redefines: &[String],
) -> Vec<String> {
    specializes
        .iter()
        .chain(redefines)
        .filter(|source| source.contains("::"))
        .filter(|source| !type_refs.iter().any(|type_ref| type_ref == *source))
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn is_implemented_core_feature(owner: &str, declared_name: &str) -> bool {
    matches!(
        (owner, declared_name),
        ("KerML::Root::Element", "documentation")
            | ("KerML::Root::Element", "isLibraryElement")
            | ("KerML::Root::Element", "name")
            | ("KerML::Root::Element", "ownedElement")
            | ("KerML::Root::Element", "owner")
            | ("KerML::Root::Element", "owningNamespace")
            | ("KerML::Root::Element", "qualifiedName")
            | ("KerML::Root::Element", "shortName")
            | ("KerML::Root::Documentation", "documentedElement")
            | ("KerML::Root::Import", "importedElement")
            | ("KerML::Root::Membership", "memberElementId")
            | ("KerML::Root::Namespace", "member")
            | ("KerML::Root::Namespace", "membership")
            | ("KerML::Root::Relationship", "relatedElement")
            | ("KerML::Root::AnnotatingElement", "annotation")
            | ("KerML::Core::Feature", "chainingFeature")
            | ("KerML::Core::Feature", "crossFeature")
            | ("KerML::Core::Feature", "featureTarget")
            | ("KerML::Core::Feature", "featuringType")
            | ("KerML::Core::Type", "differencingType")
            | ("KerML::Core::Type", "featureMembership")
            | ("KerML::Core::Type", "intersectingType")
            | ("KerML::Core::Type", "isConjugated")
            | ("KerML::Core::Type", "unioningType")
            | ("KerML::Kernel::Flow", "payloadType")
            | ("KerML::Kernel::Flow", "sourceOutputFeature")
            | ("KerML::Kernel::Flow", "targetInputFeature")
            | ("SysML::Systems::RequirementDefinition", "text")
            | ("SysML::Systems::RequirementUsage", "text")
            | ("SysML::Systems::Usage", "isReference")
    )
}

fn string_property(
    element: &KirElement,
    metadata: Option<&serde_json::Map<String, Value>>,
    key: &str,
) -> Option<String> {
    element
        .properties
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            element
                .properties
                .get(key)
                .and_then(Value::as_array)
                .and_then(|values| values.first())
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .or_else(|| {
            metadata?
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_string)
        })
}

fn string_list_property(element: &KirElement, key: &str) -> Vec<String> {
    match element.properties.get(key) {
        Some(Value::String(value)) => vec![value.clone()],
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn split_library_elements(elements: Vec<KirElement>) -> (Vec<KirElement>, Vec<KirElement>) {
    let mut kernel_elements = Vec::new();
    let mut sysml_elements = Vec::new();

    for element in elements {
        if is_kernel_element(&element) {
            kernel_elements.push(element);
        } else {
            sysml_elements.push(element);
        }
    }

    (kernel_elements, sysml_elements)
}

fn is_kernel_element(element: &KirElement) -> bool {
    element
        .properties
        .get("pilot_library_group")
        .and_then(Value::as_str)
        == Some("Kernel Libraries")
        || element
            .properties
            .get("metadata")
            .and_then(Value::as_object)
            .and_then(|metadata| metadata.get("pilot_library_group"))
            .and_then(Value::as_str)
            == Some("Kernel Libraries")
}

fn split_document(
    library_id: &str,
    note: &str,
    source_path: &str,
    elements: Vec<KirElement>,
) -> KirDocument {
    let metadata = BTreeMap::from([
        ("element_count".to_string(), json!(elements.len())),
        (
            "generator".to_string(),
            json!("cargo run -p mercurio-tools --bin generate_language_baselines"),
        ),
        ("kir_schema_version".to_string(), json!(KIR_SCHEMA_VERSION)),
        ("library_id".to_string(), json!(library_id)),
        ("library_version".to_string(), json!("0.0.0-bootstrap")),
        ("note".to_string(), json!(note)),
        ("source_path".to_string(), json!(source_path)),
    ]);

    KirDocument { metadata, elements }
}

struct Sha256 {
    state: [u32; 8],
    length_bits: u64,
    buffer: Vec<u8>,
}

impl Sha256 {
    fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            length_bits: 0,
            buffer: Vec::new(),
        }
    }

    fn update(&mut self, bytes: &[u8]) {
        self.length_bits = self.length_bits.wrapping_add((bytes.len() as u64) * 8);
        let mut input = bytes;

        if !self.buffer.is_empty() {
            let needed = 64 - self.buffer.len();
            let take = needed.min(input.len());
            self.buffer.extend_from_slice(&input[..take]);
            input = &input[take..];
            if self.buffer.len() == 64 {
                let block = <[u8; 64]>::try_from(self.buffer.as_slice()).expect("full block");
                self.compress(&block);
                self.buffer.clear();
            }
        }

        while input.len() >= 64 {
            let block = <[u8; 64]>::try_from(&input[..64]).expect("full block");
            self.compress(&block);
            input = &input[64..];
        }

        self.buffer.extend_from_slice(input);
    }

    fn finalize(mut self) -> [u8; 32] {
        self.buffer.push(0x80);
        while self.buffer.len() % 64 != 56 {
            self.buffer.push(0);
        }
        self.buffer
            .extend_from_slice(&self.length_bits.to_be_bytes());
        let blocks = self
            .buffer
            .chunks(64)
            .map(<[u8; 64]>::try_from)
            .collect::<Result<Vec<_>, _>>()
            .expect("sha256 padding yields full blocks");
        for block in blocks {
            self.compress(&block);
        }
        let mut out = [0u8; 32];
        for (index, value) in self.state.iter().enumerate() {
            out[index * 4..index * 4 + 4].copy_from_slice(&value.to_be_bytes());
        }
        out
    }

    fn compress(&mut self, block: &[u8]) {
        const K: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
            0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
            0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
            0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
            0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
            0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
            0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
            0xc67178f2,
        ];
        let mut w = [0u32; 64];
        for (index, chunk) in block.chunks_exact(4).take(16).enumerate() {
            w[index] = u32::from_be_bytes(chunk.try_into().expect("four bytes"));
        }
        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7)
                ^ w[index - 15].rotate_right(18)
                ^ (w[index - 15] >> 3);
            let s1 = w[index - 2].rotate_right(17)
                ^ w[index - 2].rotate_right(19)
                ^ (w[index - 2] >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[index])
                .wrapping_add(w[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        for (slot, value) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::sha256_hex;

    #[test]
    fn sha256_hex_matches_empty_digest() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
