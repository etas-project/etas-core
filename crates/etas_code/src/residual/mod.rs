//! Shared, engine-independent residual-check identities.
//!
//! These identities describe checked inputs; they are not installation evidence
//! and do not authorize an effect or permit a candidate to execute.

pub mod binding;
pub mod identity;
pub mod model;
pub mod monitor;
pub mod pattern;
pub mod plan;
pub mod predicate;
pub mod target;
pub mod type_binding;
pub mod type_template;
pub mod validation;
