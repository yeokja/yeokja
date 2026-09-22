//! Inline markup exchanged with the model as numbered tags.
//!
//! The model translates prose; the markup around it is written back by a
//! serializer that knows the markup's rules. See
//! `docs/superpowers/specs/2026-09-22-inline-tag-transport-design.md`.

pub mod audit;
pub mod markdown;
