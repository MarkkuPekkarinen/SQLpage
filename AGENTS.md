Core Concept: User writes .sql files, SQLPage executes queries, results mapped to handlebars UI components,
HTML streamed to client

## Validation

### When working on rust code
Mandatory formatting (rust): `cargo fmt --all`
Mandatory linting: `cargo clippy --all-targets --all-features -- -D warnings`

### When working on css or js
Frontend formatting: `npm run format`
Mandatory frontend validation: `npm test`

More about testing: see [github actions](./.github/workflows/ci.yml).
Contributor setup and validation: see [CONTRIBUTING.md](./CONTRIBUTING.md). Module overview: see [src/lib.rs](./src/lib.rs); architecture diagram: [docs/architecture-detailed.png](./docs/architecture-detailed.png).

NEVER reformat/lint/touch files unrelated to your task. Always run tests/lints/format before stopping when you changed code.

### Testing

```
cargo test # tests with in-memory SQLite by default
```

For other databases, see [docker testing setup](./docker-compose.yml)

```
docker compose up --wait mssql # or postgres, mysql, mariadb, oracle
DATABASE_URL='mssql://root:Password123!@localhost/sqlpage' cargo test
```

ODBC tests require the database-specific ODBC driver on the host; starting the container is not sufficient. See the PostgreSQL ODBC and Oracle matrix entries in [CI](./.github/workflows/ci.yml) for driver setup and connection strings. On Linux and macOS, `cargo test --features odbc-static` matches CI's static unixODBC linking.

For dynamic frontend changes, run the Playwright tests under `tests/end-to-end/` as described in [CONTRIBUTING.md](./CONTRIBUTING.md). For examples containing `test.hurl`, run `scripts/test-examples-hurl.sh <example-path>`.

### Documentation

Components and functions are documented in [official-site migrations](./examples/official-site/sqlpage/migrations/). Edit the existing migration for an existing entity; add an appropriately ordered migration for a new entity. The official-site database is recreated from migrations on each deployment.

official documentation website sql tables:
  - `parameter_type(type)`
  - `component(name,description,icon,introduced_in_version)` -- icon name from tabler icon
  - `parameter(top_level BOOLEAN, name, component REFERENCES component(name), description, description_md, type, optional BOOLEAN)` parameter types: BOOLEAN, COLOR, HTML, ICON, INTEGER, JSON, REAL, TEXT, TIMESTAMP, URL
  - `example(component REFERENCES component(name), description, properties JSON)`
  - `sqlpage_functions(name,icon,description_md,return_type,introduced_in_version)`
  - `sqlpage_function_parameters(function,index,name,description_md,type)`

#### Project Conventions

- Built-in UI component templates: `sqlpage/templates/*.handlebars`; header/control components: `src/render.rs`.
- SQLPage functions: one `async fn` module under `src/webserver/database/sqlpage_functions/functions/`, registered with `sqlpage_functions!` in `functions.rs`.
- [Configuration](./configuration.md): see [AppConfig](./src/app_config.rs)
- Routing: file-based in `src/webserver/routing.rs`. Missing paths use the nearest ancestor `404.sql`; without one, HTML uses `src/default_404.sql` and other formats receive a plain-text 404.
- Follow patterns from similar modules before introducing new abstractions.
- Frontend: see [css](./sqlpage/sqlpage.css) and [js](./sqlpage/sqlpage.js).
