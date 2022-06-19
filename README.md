# Nebula (A12)

## Building

Requirements:

- Node
- Rust (stable toolchain)
- Cargo

First in the `frontend` directory:

```bash
npm install
npm run build
```

Then in the `backend` directory:

```bash
cargo build --release
```

## Deploying

The backend binary serves the frontend content and has been tested on both ARM64 macOS and x86_64 Linux hosts. It is configured using the following environment variables:

Name | Required | Description | Example
-|-|-|-
`FQDN` | Y | Fully-qualified Domain Name server will be available at; determines the `host` field for users, posts and communities. | `nebula0.herokuapp.com`
`WEB_ADDR` | Y | Address HTTP server will bind to | `0.0.0.0:8080`
`DATABASE_URL` | Y | URL of a PostgreSQL instance including username and password | `postgres://localhost:5432/nebula`
`DIST_PATH` | Y | Path to frontend content to serve | `../frontend/dist`
`SECRET` | Y | 512-bit base64 encoded random data to be used as the JSON Web Token secret | `MPJ0HkSe...`
`PRIVKEY` | Y | PKCS#8 RSA private key | `MIIJRAIB...`
`RUST_LOG` | N | Sets the log output level | `debug`

The use of a `.env` file is supported as an alternative to environment variables.

The [`SQLx CLI`](https://github.com/launchbadge/sqlx/tree/master/sqlx-cli) tool is used to manage migrations. The following command will prepare the database in the `DATABASE_URL` environment variable by creating the database and applying all migrations:

```bash
sqlx database create
```

## Testing

Backend tests can be run with `cargo test` and require all the above environment variables be set as the integration tests make requests against a full backend instance.

**WARNING**: the database is wiped before running tests, do _not_ run `cargo test` with any important database set as the environment variable.

## Structure

The backend is implemented as library and small binary application which calls a library entrypoint. This is done to make creating a backend instance during testing easier.

File | | | Description
-|-|-|-
`backend` | | | Backend implementation
^ | `migrations` | | SQL migration files
^ | `Cargo.toml` | | Cargo package manifest
^ | `Cargo.lock` | | Cargo dependency lockfile
^ | `src` | | Rust source
^ | ^ | `main.rs` | Binary application entrypoint
^ | ^ | `lib.rs` | Library backend source file
^ | ^ | `fed` | Federation API routes
^ | ^ | `internal` | Internal API routes
^ | ^ | `middleware` | Authentication and federation security middleware
^ | ^ | `models` | Structures used for database and API interactions
