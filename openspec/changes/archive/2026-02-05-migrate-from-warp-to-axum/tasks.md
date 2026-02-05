# Tasks: Migration from Warp to Axum

## 1. Update Dependencies

- [x] 1.1 Replace `warp` dependency with `axum` in `Cargo.toml`
- [x] 1.2 Add necessary dependencies for Axum (`tower`, `tower-http`, etc.)
- [x] 1.3 Run `cargo build` to ensure dependencies are correctly resolved

## 2. Update Imports and Handlers

- [x] 2.1 Replace `warp` imports with `axum` imports in `src/commands/serve/runner/mod.rs`
- [x] 2.2 Adapt handlers to use Axum traits (`IntoResponse`, `FromRequest`)
- [x] 2.3 Update the `index` handler to use Axum
- [x] 2.4 Update the `update` handler to use Axum
- [x] 2.5 Update the `status` handler to use Axum
- [x] 2.6 Update the `set_status_to_error` handler to use Axum
- [x] 2.7 Update the `unites_legales` handler to use Axum
- [x] 2.8 Update the `etablissements` handler to use Axum

## 3. Update Error Handling

- [x] 3.1 Modify error handling in `src/commands/serve/runner/error.rs` to use `axum::response::Result`
- [x] 3.2 Ensure all errors are properly reported to Sentry
- [x] 3.3 Update the `Error` type to be compatible with Axum

## 4. Configure Router

- [x] 4.1 Replace the Warp router configuration with `axum::Router`
- [x] 4.2 Configure all routes (`/`, `/v3`, `/v3/unites_legales/<siren>`, etc.)
- [x] 4.3 Configure middleware for CORS and tracing
- [x] 4.4 Ensure all routes and middleware are correctly configured

## 5. Integrate Sentry

- [x] 5.1 Configure Sentry middleware for automatic error tracking
- [x] 5.2 Ensure all errors and performance data are captured and reported
- [x] 5.3 Test Sentry integration with sample errors

## 6. Testing

- [x] 6.1 Test the health check endpoint (`GET /`)
- [x] 6.2 Test the index endpoint (`GET /v3`)
- [x] 6.3 Test the unit legale endpoint (`GET /v3/unites_legales/<siren>`)
- [x] 6.4 Test the etablissement endpoint (`GET /v3/etablissements/<siret>`)
- [x] 6.5 Test the admin update endpoint (`POST /admin/update`)
- [x] 6.6 Test the admin status endpoint (`GET /admin/update/status`)
- [x] 6.7 Test the admin status error endpoint (`POST /admin/update/status/error`)
- [x] 6.8 Test error handling for invalid SIREN/SIRET
- [x] 6.9 Test error handling for missing/invalid API keys

