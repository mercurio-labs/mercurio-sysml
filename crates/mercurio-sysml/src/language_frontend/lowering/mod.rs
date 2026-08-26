//! Explicit lowering layer for language frontends.
//!
//! This module is the target home for the phases between parsed ASTs and final
//! KIR emission. The initial extraction keeps the existing resolver/transpiler
//! behavior intact while giving the phases stable module boundaries.

pub mod collect;
pub mod elaborate;
pub mod emit;
pub mod imports;
pub mod indexes;
pub mod ir;
pub mod mappings;
pub mod names;
pub mod pilot_evidence;
pub mod policy;
pub mod resolve;
pub mod rules;
pub mod semantic_actions;
pub mod semantic_defaults;
pub mod semantic_properties;
