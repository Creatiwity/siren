# Proposal

## Why

The migration from Warp to Axum is motivated by several reasons:
- **Axum** has become the most popular and maintained web framework in the Rust ecosystem.
- Better integration with the Tokio and Tower ecosystem.
- Better error handling and middleware management.
- Better documentation and active community.
- Built-in integration with Sentry for improved error tracking and monitoring.

## What Changes

- Replacement of all `warp` dependencies and imports with `axum`.
- Adaptation of handlers to use Axum traits and types.
- Update error handling to use `axum::response::Result`.
- Configuration of the router with `axum::Router`.
- Integration of Sentry with Axum for seamless error tracking.

## Capabilities

### New Capabilities
- `http-server-axum`: Migration of the HTTP server from Warp to Axum.
- `sentry-integration-axum`: Integration of Sentry with Axum for error tracking.

### Modified Capabilities
- None (existing endpoints and functionality remain unchanged, only internal implementation changes).

## Impact

- `src/commands/serve/runner/mod.rs`: Modify imports, handlers, and router.
- `src/commands/serve/runner/error.rs`: Adapt error handling for Axum.
- `Cargo.toml`: Replace the `warp` dependency with `axum` and its associated dependencies (`tower`, `tower-http`, etc.).