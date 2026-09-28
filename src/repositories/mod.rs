//! Thin typed data-access layer on top of OxiMod.
//!
//! Each module wraps the OxiMod model operations one entity needs so that
//! services and handlers never assemble filters or update documents
//! themselves. All functions run against the process-global OxiMod client
//! initialized at startup.

pub mod about;
pub mod article;
pub mod contact;
pub mod experience;
pub mod hero;
pub mod media;
pub mod project;
pub mod skills;
pub mod social;
pub mod user;
