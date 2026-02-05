# Design: Add Scalar API Documentation

## Context

The current SIREN API lacks comprehensive and interactive documentation. Adding Scalar API documentation will provide a modern and user-friendly interface for developers to explore and understand the API. This design outlines the approach to integrate Scalar with the existing Axum HTTP server.

## Goals / Non-Goals

**Goals:**
- Provide an interactive and well-documented reference for the SIREN API.
- Improve the developer experience by making it easier to understand and use the API.
- Integrate Scalar with the existing Axum HTTP server.
- Ensure the documentation is accessible and easy to use.

**Non-Goals:**
- Refactor the existing API endpoints or business logic.
- Add new endpoints or modify existing endpoint behaviors.
- Optimize performance beyond what Scalar provides by default.

## Decisions

### Decision 1: Use Scalar for API Documentation

**Rationale:**
- Scalar provides a modern and user-friendly interface for API documentation.
- It offers interactive features such as search, example requests, and responses.
- Scalar is well-integrated with Axum, making it easy to set up and configure.
- It supports OpenAPI specifications, which are widely used and understood.

**Alternatives Considered:**
- **Swagger UI**: While popular, Swagger UI is less modern and user-friendly compared to Scalar.
- **Redoc**: Redoc is another option, but it lacks the interactive features of Scalar.

### Decision 2: Serve Scalar on `/api-reference`

**Rationale:**
- `/api-reference` is a clear and intuitive path for API documentation.
- It avoids conflicts with existing endpoints.
- It is easy to remember and access.

**Alternatives Considered:**
- **`/docs`**: While common, `/docs` can be ambiguous and may conflict with other documentation.
- **`/scalar`**: While specific, `/scalar` is less intuitive for users who are not familiar with Scalar.

### Decision 3: Use Existing Axum Router and Middleware

**Rationale:**
- Using the existing Axum router and middleware ensures consistency and reduces complexity.
- It allows Scalar to benefit from existing middleware such as CORS and tracing.
- It simplifies the integration process.

**Alternatives Considered:**
- **Separate Router**: While possible, a separate router would add unnecessary complexity and duplication.

## Risks / Trade-offs

### Risk 1: Increased Complexity

**Description:** Adding Scalar API documentation may increase the complexity of the project.

**Mitigation:**
- Keep the integration simple and well-documented.
- Use existing infrastructure and patterns where possible.

### Risk 2: Performance Impact

**Description:** The Scalar API documentation may introduce performance overhead.

**Mitigation:**
- Monitor performance before and after the integration.
- Optimize the configuration and setup of Scalar to minimize overhead.

### Risk 3: Learning Curve

**Description:** The team may need time to familiarize themselves with Scalar.

**Mitigation:**
- Provide documentation and examples for common tasks in Scalar.
- Offer training or workshops to share knowledge.

## Implementation Plan

### Step 1: Add Dependencies

- Add the necessary dependencies for Scalar (`scalar-axum`, `scalar-api-reference`) to `Cargo.toml`.

### Step 2: Configure Scalar

- Configure Scalar to generate documentation for all existing endpoints.
- Set up the Scalar API reference with the appropriate configuration options.

### Step 3: Integrate Scalar with Axum

- Add a new route to serve the Scalar API documentation on `/api-reference`.
- Ensure the Scalar API documentation uses the existing Axum router and middleware.

### Step 4: Test the Documentation

- Test that the Scalar API documentation is accessible and functional.
- Verify that the documentation includes all existing endpoints.
- Ensure the documentation is interactive and user-friendly.

### Step 5: Deploy the Documentation

- Deploy the Scalar API documentation to a staging environment for further testing.
- Monitor for any issues and address them promptly.
- Deploy to production with a rollback plan in case of major issues.

## Open Questions

- **Performance Impact**: What is the expected performance impact of adding Scalar, and how can it be minimized?
- **Configuration Options**: What configuration options should be used for Scalar to best fit the needs of the SIREN API?
- **Customization**: How can the Scalar API documentation be customized to match the branding and style of the SIREN API?