//! OxiMod models mirroring the existing MongoDB collections.
//!
//! The previous application accessed MongoDB through Prisma without any
//! `@@map` overrides, so every collection is named exactly like its Prisma
//! model (PascalCase, singular): `About`, `Contact`, `Experience`, `Hero`,
//! `Project`, `Skills`, `Social`, `User`, `Media`, `Article`.
//!
//! Stored field names are camelCase; the Rust structs use idiomatic
//! snake_case with Serde renames so the BSON representation matches the
//! existing documents byte for byte. Do not change field renames, enum
//! string values, or collection names without a deliberate data migration.
//!
//! OxiMod resolves the database from the compile-time `#[db(...)]`
//! attribute; every model here uses [`crate::config::DB_NAME`]'s value
//! (`"portfolio"`), which startup validates against `DATABASE_URL`.
//!
//! None of the models declare `#[index(...)]` on purpose: the unique
//! indexes on `User.email` and `Article.slug` already exist in the
//! database under Prisma-chosen names (for example `Article_slug_key`),
//! and declaring equivalent indexes under different names would make
//! MongoDB reject index creation during saves.

mod about;
mod article;
mod contact;
mod experience;
mod hero;
mod media;
mod project;
mod skills;
mod social;
mod user;

pub use about::About;
pub use article::Article;
pub use contact::{Contact, ContactStatus};
pub use experience::Experience;
pub use hero::Hero;
pub use media::{Media, MediaType};
pub use project::Project;
pub use skills::Skills;
pub use social::{Social, SocialName};
pub use user::{Role, User};
