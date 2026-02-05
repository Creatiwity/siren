# Proposal: Add Scalar API Documentation

## Why

Adding Scalar API documentation will provide an interactive and well-documented reference for the SIREN API. This will improve the developer experience by making it easier to understand and use the API. Scalar offers a modern and user-friendly interface for API documentation, which will help both internal and external developers to integrate with the SIREN API more efficiently.

## What Changes

- Add Scalar API documentation to the SIREN API.
- Configure Scalar to generate documentation for all existing endpoints.
- Integrate Scalar with the Axum HTTP server.
- Add a new route to serve the Scalar API documentation.

## Capabilities

### New Capabilities
- **scalar-api-documentation**: Add Scalar API documentation to the SIREN API.

### Modified Capabilities
- None (existing endpoints and functionality remain unchanged).

## Impact

- `src/commands/serve/runner/mod.rs`: Add route for Scalar API documentation.
- `Cargo.toml`: Add dependencies for Scalar (`scalar-axum`, `scalar-api-reference`).
- `openspec/changes/add-scalar-api-documentation/`: New change for adding Scalar API documentation.