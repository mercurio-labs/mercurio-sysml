use std::path::PathBuf;

use mercurio_core::frontend::ast::{Declaration, QualifiedName};
use mercurio_sysml::parse_sysml;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: inspect_sysml_parse <input.sysml>")?;
    let text = std::fs::read_to_string(PathBuf::from(path))?;
    let module = parse_sysml(&text)?;

    for member in &module.members {
        dump_decl(member, 0);
    }
    Ok(())
}

fn dump_decl(declaration: &Declaration, depth: usize) {
    let pad = "  ".repeat(depth);
    if let Some(definition) = declaration.as_definition_like() {
        println!("{pad}{} def {}", definition.keyword, definition.name);
        for member in &definition.members {
            dump_decl(member, depth + 1);
        }
        return;
    }
    if let Some(usage) = declaration.as_usage_like() {
        println!(
            "{pad}{} {}{}{}",
            usage.keyword,
            usage.name,
            display_type(usage.ty.as_ref()),
            display_usage_relations(&usage.specializes, &usage.subsets, &usage.redefines)
        );
        for member in &usage.body_members {
            dump_decl(member, depth + 1);
        }
        return;
    }

    match declaration {
        Declaration::Package(package) => {
            println!("{pad}package {}", package.name.as_dot_string());
            for member in &package.members {
                dump_decl(member, depth + 1);
            }
        }
        Declaration::Import(import) => {
            println!("{pad}import {}", display_name(&import.path));
        }
        Declaration::Alias(alias) => {
            println!(
                "{pad}alias {} for {}",
                alias.name,
                display_name(&alias.target)
            );
        }
        _ => unreachable!("definition-like and usage-like declarations are handled above"),
    }
}

fn display_name(name: &QualifiedName) -> String {
    name.as_colon_string()
}

fn display_type(name: Option<&QualifiedName>) -> String {
    name.map(|name| format!(" : {}", name.as_colon_string()))
        .unwrap_or_default()
}

fn display_usage_relations(
    specializes: &[QualifiedName],
    subsets: &[QualifiedName],
    redefines: &[QualifiedName],
) -> String {
    let mut parts = Vec::new();
    if !specializes.is_empty() {
        parts.push(format!(" :> {}", display_names(specializes)));
    }
    if !subsets.is_empty() {
        parts.push(format!(" subsets {}", display_names(subsets)));
    }
    if !redefines.is_empty() {
        parts.push(format!(" :>> {}", display_names(redefines)));
    }
    parts.concat()
}

fn display_names(names: &[QualifiedName]) -> String {
    names
        .iter()
        .map(QualifiedName::as_colon_string)
        .collect::<Vec<_>>()
        .join(", ")
}
