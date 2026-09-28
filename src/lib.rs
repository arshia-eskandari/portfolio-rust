//! Server-rendered portfolio website built with Axum, Askama and OxiMod.
//!
//! The crate is organized in layers:
//!
//! - [`models`] — OxiMod models mirroring the existing MongoDB collections
//!   (BSON-compatible with the documents written by the previous Prisma app).
//! - [`repositories`] — thin typed data-access functions on top of OxiMod.
//! - [`services`] — domain logic: cached public data, article rendering,
//!   authentication, contact handling, media/S3.
//! - [`routes`] — HTTP handlers (public site, auth, admin).
//! - [`middleware`] — authentication guard and security headers.
//! - [`seo`] — page metadata, JSON-LD, sitemap, robots and the Atom feed.

pub mod app;
pub mod config;
pub mod error;
pub mod markdown;
pub mod middleware;
pub mod models;
pub mod repositories;
pub mod routes;
pub mod seo;
pub mod services;
pub mod slug;
