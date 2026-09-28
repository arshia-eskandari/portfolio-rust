# arshiaeskandari.com — Rust Portfolio

Server-rendered portfolio website written in Rust. It replaces the previous
Next.js implementation while operating against the **same MongoDB database
and documents** — no data migration was performed or required.

The site itself is part of the portfolio: it runs on
[Axum](https://github.com/tokio-rs/axum), renders with
[Askama](https://askama.readthedocs.io) and talks to MongoDB through
[OxiMod](https://crates.io/crates/oximod), a schema-aware MongoDB ODM for
Rust.

## Why this stack

- **Axum 0.8** — mature, tower-based, minimal-magic HTTP framework; every
  route is a plain async function.
- **Askama** — compile-time HTML templates: template errors are build
  errors, output is escaped by default, and rendering costs no runtime
  parsing. Integrated with Axum via `askama_web` (the maintained
  replacement for the deprecated `askama_axum`).
- **OxiMod 1.x** — typed models over the official MongoDB driver: derived
  fluent builders, typed queries/updates whose field paths follow Serde
  renames, and direct driver access when needed. Used for every database
  operation in this application.
- **comrak + syntect** — GitHub-flavored Markdown rendered fully on the
  server with class-based syntax highlighting; no client-side highlighting
  bundle, no `unsafe-inline` CSP exceptions.

There is no Node.js anywhere: CSS is hand-written, the only JavaScript is
one small progressive-enhancement file, and the deployed artifact is a
single Rust binary plus the `static/` and (embedded) template assets.

## Directory structure

```
src/
  main.rs            startup: env, tracing, Mongo client, S3, serve
  app.rs             AppState, caches, rate limiters, router assembly
  config.rs          environment configuration + DB-name validation
  error.rs           AppError -> intentional HTTP responses
  markdown.rs        comrak/syntect rendering + plain-text excerpts
  slug.rs            article slug generation (ported from lib/slug.ts)
  models/            OxiMod models (BSON-compatible with Prisma data)
  repositories/      typed data access per collection
  services/          domain logic: portfolio data, articles, auth,
                     contact, media/S3, TTL cache, date formatting
  routes/            public pages, auth, contact, SEO endpoints, admin/
  middleware/        admin JWT guard, security headers + origin check
  seo/               page metadata, JSON-LD, sitemap, Atom feed
templates/           Askama templates (public site + admin)
static/              CSS (main + generated highlight theme), JS, favicon
tests/               BSON-compatibility and routing integration tests
examples/            gen_highlight_css (regenerates highlight.css),
                     verify_db / verify_s3 (read-only live checks, see below)
```

## MongoDB compatibility

The previous app used Prisma without collection remapping, so the
collections are PascalCase singular: `About`, `Contact`, `Experience`,
`Hero`, `Project`, `Skills`, `Social`, `User`, `Media`, `Article`. The
Rust models keep:

- camelCase stored field names (Serde renames; structs stay snake_case),
- `_id` ObjectIds (omitted on insert so MongoDB assigns them),
- BSON dates for all timestamps,
- Prisma enum strings (`PENDING`, `ADMIN`, `LINKEDIN`, `IMAGE`, …),
- nullable/missing optionals and string arrays,
- `Project.order` as `i64`: the existing documents store it as BSON Int64,
  so admin saves write the same type back (Int32 values still load).

`tests/serialization.rs` proves round-trips against literal Prisma-shaped
documents. No model declares `#[index(...)]` on purpose: the unique
indexes on `User.email` and `Article.slug` already exist under
Prisma-chosen names, and re-declaring them under different names would
make MongoDB reject index creation.

**Database name:** OxiMod fixes the database name at compile time
(`#[db("portfolio")]`, mirrored by `config::DB_NAME`). Startup parses the
default database out of `DATABASE_URL` and refuses to boot on a mismatch.
If your database is named differently, update the `#[db(...)]` attribute
in `src/models/*.rs` and `DB_NAME` in `src/config.rs`.

Two read-only examples check a real deployment without writing anything
(they load `.env`, print no secrets, and never print contact messages or
password hashes):

```bash
cargo run --example verify_db   # databases, collections, BSON shape of every
                                # document, per-document model deserialization,
                                # index names, the app's own typed queries
cargo run --example verify_s3   # bucket reachable, bucket region matches
                                # AWS_REGION, every Media object exists in S3
```

Article slugs use the exact generation rules of the old
`src/lib/slug.ts` (NFKD fold, dash-collapsing, 6-char suffix from the
document id), so editing titles produces identical slugs and existing
slugs never change spontaneously.

## Environment variables

Copy `.env.example` to `.env`. Values kept from the old deployment:
`DATABASE_URL`, `JWT_SECRET`, `AWS_ACCESS_KEY_ID`,
`AWS_SECRET_ACCESS_KEY`, `AWS_REGION`, `S3_BUCKET_NAME`,
`RECAPTCHA_SECRET_KEY`.

Renamed/new (documented in `.env.example`):

| Old | New | Notes |
|-----|-----|-------|
| `NEXT_PUBLIC_ENABLE_RECAPTCHA` | `ENABLE_RECAPTCHA` | old name still read as fallback |
| `HOST_NAME` (image host) | `SITE_BASE_URL` | canonical public origin for SEO |
| — | `RECAPTCHA_SITE_KEY` | v3 site key rendered into the page |
| — | `APP_HOST`, `APP_PORT`, `APP_ENV`, `RUST_LOG` | server/runtime settings |

## Running locally

```bash
cp .env.example .env   # fill in DATABASE_URL, JWT_SECRET, S3 settings
cargo run              # http://localhost:3000
```

The MongoDB driver connects lazily, so the server boots even if the
database is briefly unavailable. `GET /health` is the liveness endpoint.

## Tests and quality gates

```bash
cargo fmt --check
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Tests never touch a real database: BSON compatibility is proven against
in-memory documents, and routing tests drive the router directly. For a
live, read-only check against a real database and bucket use the
`verify_db` and `verify_s3` examples described above.

## Production build

```bash
cargo build --release          # target/release/portfolio
# or
docker build -t portfolio .
```

The binary serves `static/` itself (precompressed variants are picked up
when present), performs graceful shutdown on SIGINT/SIGTERM, and honors
`APP_HOST`/`APP_PORT`.

## Media / S3

Uploads are validated by extension **and** magic bytes (`infer`), stored
under `media/{uuid}-{sanitized-name}` with an explicit content type, and
capped at 50 MB. Deleting media first removes every reference to its URL
(experience recommendation letters, project links + parallel titles,
project media, about resume/portrait, article banners) before deleting
the S3 object and the record — so nothing on the site ever points at a
deleted object.

## SEO implementation

- All public content is in the initial server-rendered HTML.
- Per-page `<title>`, meta description, canonical URL, Open Graph and
  Twitter cards; `article:*` metadata on article pages.
- JSON-LD: `Person` + `WebSite` on the homepage, `BlogPosting` +
  `BreadcrumbList` on articles (only truthful, stored data).
- `/sitemap.xml` (static pages + all articles with `lastmod`),
  `/robots.txt` (blocks `/admin`, `/login`; points at the sitemap),
  `/feed.xml` (Atom).
- `/article/:slug` → `/articles/:slug` as a real **301**.
- Custom 404, `noindex` on auth/admin pages, semantic landmarks and
  heading hierarchy, `prefers-reduced-motion` respected.

## Security

- Admin JWT (HS256, 1 h) in an `HttpOnly` + `SameSite=Strict` cookie
  (`Secure` in production) — format-compatible with the previous `jose`
  tokens, and password verification matches the existing stored SHA-512
  digests, so the existing admin account keeps working unchanged.
- Admin role re-verified server-side on every `/admin` request.
- Cross-origin `POST`s rejected by an Origin check (CSRF defense in
  depth on top of `SameSite=Strict`).
- Security headers: CSP (no inline script/style), `X-Content-Type-Options`,
  `X-Frame-Options`/`frame-ancestors`, `Referrer-Policy`, HSTS in
  production.
- Contact form: honeypot + per-IP rate limiting + optional reCAPTCHA v3;
  login is rate-limited too.
- Markdown rendered with raw HTML escaped and dangerous URL schemes
  suppressed; JSON-LD output escapes `<`.
- Request body limits (2 MB default, 12 MB admin forms, 60 MB uploads).

## Deployment

Any host that can run a Linux binary or the provided Docker image works.
Terminate TLS in front of the app (it speaks plain HTTP), set
`APP_ENV=production` and `SITE_BASE_URL=https://arshiaeskandari.com`, and
forward the client IP via `X-Forwarded-For` so rate limiting keys on real
addresses.
