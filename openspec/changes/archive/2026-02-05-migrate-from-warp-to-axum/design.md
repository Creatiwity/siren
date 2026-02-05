# Design: Migration from Warp to Axum

## Context

The current implementation uses Warp as the web framework for the HTTP server. While Warp is functional, Axum offers better integration with the Tokio ecosystem, improved error handling, and built-in support for Sentry. This design outlines the approach to migrate the HTTP server from Warp to Axum while maintaining all existing functionality.

## Goals / Non-Goals

**Goals:**
- Migrate the HTTP server from Warp to Axum.
- Maintain all existing endpoints and their functionality.
- Integrate Sentry with Axum for improved error tracking.
- Ensure minimal downtime and a smooth transition.

**Non-Goals:**
- Refactor the business logic or database interactions.
- Add new endpoints or modify existing endpoint behaviors.
- Optimize performance beyond what Axum provides by default.

## Decisions

### Decision 1: Use Axum for HTTP Server

**Rationale:**
- Axum is the most popular and maintained web framework in the Rust ecosystem.
- Better integration with Tokio and Tower, which are already used in the project.
- Built-in support for Sentry, simplifying error tracking and monitoring.
- More intuitive and flexible routing and middleware system.

**Alternatives Considered:**
- **Actix Web**: While powerful, it has a different architecture and would require more significant changes.
- **Rocket**: Not as well-integrated with Tokio and lacks built-in Sentry support.

### Decision 2: Maintain Existing Endpoints

**Rationale:**
- Ensures backward compatibility for all clients using the API.
- Reduces the risk of breaking changes and simplifies testing.
- Allows for a gradual migration without disrupting existing functionality.

**Alternatives Considered:**
- **Refactor Endpoints**: While tempting, this would introduce unnecessary risk and complexity.

### Decision 3: Integrate Sentry with Axum

**Rationale:**
- Axum has built-in support for Sentry, making integration straightforward.
- Improves error tracking and monitoring, which is critical for production environments.
- Provides better context and debugging information for errors.

**Alternatives Considered:**
- **Custom Error Handling**: Would require more effort and might not provide the same level of detail as Sentry.

## Risks / Trade-offs

### Risk 1: Breaking Changes

**Description:** Despite efforts to maintain compatibility, there is a risk of introducing breaking changes during the migration.

**Mitigation:**
- Thorough testing of all endpoints to ensure they behave as expected.
- Gradual rollout with monitoring to catch any issues early.

### Risk 2: Performance Impact

**Description:** The migration to Axum might introduce performance overhead, especially with additional middleware like Sentry.

**Mitigation:**
- Performance testing before and after the migration.
- Optimization of middleware and error handling to minimize overhead.

### Risk 3: Learning Curve

**Description:** The team might need time to familiarize themselves with Axum, especially if they are used to Warp.

**Mitigation:**
- Documentation and examples for common tasks in Axum.
- Pair programming and code reviews to share knowledge.

## Migration Plan

### Step 1: Update Dependencies

- Replace the `warp` dependency with `axum` in `Cargo.toml`.
- Add necessary dependencies for Axum, such as `tower` and `tower-http`.

### Step 2: Update Imports and Handlers

- Replace all `warp` imports with `axum` imports in `src/commands/serve/runner/mod.rs`.
- Adapt handlers to use Axum traits (`IntoResponse`, `FromRequest`).

### Step 3: Update Error Handling

- Modify error handling in `src/commands/serve/runner/error.rs` to use `axum::response::Result`.
- Ensure all errors are properly reported to Sentry.

### Step 4: Configure Router

- Replace the Warp router configuration with `axum::Router`.
- Ensure all routes and middleware are correctly configured.

### Step 5: Integrate Sentry

- Configure Sentry middleware for automatic error tracking.
- Ensure all errors and performance data are captured and reported.

### Step 6: Testing

- Test all endpoints to ensure they work as expected.
- Verify error handling and Sentry integration.
- Performance testing to ensure no significant overhead.

### Step 7: Deployment

- Deploy the changes to a staging environment for further testing.
- Monitor for any issues and address them promptly.
- Deploy to production with a rollback plan in case of major issues.

## Open Questions

- **Performance Impact**: What is the expected performance impact of the migration, and how can it be minimized?
- **Error Handling**: Are there any specific error handling patterns in Axum that should be followed?
- **Middleware**: What middleware should be used for CORS and other common tasks?