use std::path::PathBuf;

use mercurio_core::frontend::ast::Declaration;
use mercurio_core::frontend::lexer::lex;
use mercurio_core::ir::KirDocument;
use mercurio_language_frontend::lowering::ir::ResolvedUsage;
use mercurio_language_frontend::resolver::resolve_module;
use mercurio_language_frontend::transpile::MappingBundle;
use mercurio_sysml::parse_sysml;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = std::env::args()
        .nth(1)
        .ok_or("usage: cargo run -p mercurio-tools --bin inspect_connection -- <input.sysml>")?;
    let text = std::fs::read_to_string(PathBuf::from(input))?;
    let module = parse_sysml(&text)?;
    let stdlib = KirDocument {
        metadata: Default::default(),
        elements: Vec::new(),
    };
    let mappings = MappingBundle::load()?;
    let resolved = resolve_module(&module, &stdlib, &mappings)?;

    println!("parsed members:");
    if let Some(package) = &module.package {
        for member in &package.members {
            dump_decl(member, 0);
        }
    }

    println!("resolved usages:");
    for usage in &resolved.usages {
        dump_usage(usage, 0);
    }
    for definition in &resolved.definitions {
        println!(
            "definition {} {}",
            definition.construct, definition.qualified_name
        );
        for member in &definition.members {
            dump_usage(member, 1);
        }
    }

    println!("tokens around connection lines:");
    for token in lex(&text)? {
        if (28..=31).contains(&token.span.start_line) || (38..=39).contains(&token.span.start_line)
        {
            println!(
                "line {} col {} kind {:?}",
                token.span.start_line, token.span.start_col, token.kind
            );
        }
    }

    Ok(())
}

fn dump_decl(decl: &Declaration, depth: usize) {
    let pad = "  ".repeat(depth);
    if let Some(defn) = decl.as_definition_like() {
        println!("{pad}{} def {}", defn.keyword, defn.name);
        for member in &defn.members {
            dump_decl(member, depth + 1);
        }
        return;
    }
    if let Some(usage) = decl.as_usage_like() {
        println!(
            "{pad}{} {} ref={:?}",
            usage.keyword,
            usage.name,
            usage
                .reference_target
                .as_ref()
                .map(|target| target.as_dot_string())
        );
        for member in &usage.body_members {
            dump_decl(member, depth + 1);
        }
        return;
    }

    match decl {
        Declaration::Package(pkg) => {
            println!("{pad}package {}", pkg.name.as_dot_string());
            for member in &pkg.members {
                dump_decl(member, depth + 1);
            }
        }
        Declaration::Import(import) => {
            println!("{pad}import {}", import.path.as_dot_string());
        }
        Declaration::Alias(alias) => {
            println!("{pad}alias {}", alias.name);
        }
        _ => unreachable!("definition-like and usage-like declarations are handled above"),
    }
}

fn dump_usage(usage: &ResolvedUsage, depth: usize) {
    let pad = "  ".repeat(depth);
    println!(
        "{pad}{} {} type={:?} ref={:?}",
        usage.construct, usage.qualified_name, usage.type_ref, usage.reference_target
    );
    for member in &usage.members {
        dump_usage(member, depth + 1);
    }
}
