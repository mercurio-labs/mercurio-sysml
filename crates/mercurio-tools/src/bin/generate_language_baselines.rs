use std::path::Path;

use mercurio_core::{KirDocument, default_kernel_library_path, repo_root};
use mercurio_sysml::{default_sysml_delta_library_path, default_sysml_library_path};
use mercurio_tools::split_language_baselines;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_path = default_sysml_library_path();
    let source = KirDocument::from_path(&source_path)?;
    let split = split_language_baselines(source, stable_source_path(&source_path));

    split
        .kernel
        .write_pretty_to_path(&default_kernel_library_path())?;
    split
        .sysml_delta
        .write_pretty_to_path(&default_sysml_delta_library_path())?;

    println!("wrote {}", default_kernel_library_path().display());
    println!("wrote {}", default_sysml_delta_library_path().display());
    Ok(())
}

fn stable_source_path(path: &Path) -> String {
    path.strip_prefix(repo_root())
        .map(path_to_slash_path)
        .unwrap_or_else(|_| path_to_slash_path(path))
}

fn path_to_slash_path(path: &Path) -> String {
    let mut parts = Vec::new();
    for component in path.components() {
        parts.push(component.as_os_str().to_string_lossy().to_string());
    }
    parts.join("/")
}
