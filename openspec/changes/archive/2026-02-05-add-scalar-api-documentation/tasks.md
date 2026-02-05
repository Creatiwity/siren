# Tasks: Add Scalar API Documentation

## 1. Add Dependencies

- [x] 1.1 Add `scalar-axum` dependency to `Cargo.toml`
- [x] 1.2 Add `scalar-api-reference` dependency to `Cargo.toml`
- [x] 1.3 Run `cargo build` to ensure dependencies are correctly resolved

## 2. Configure Scalar

- [ ] 2.1 Configure Scalar to generate documentation for all existing endpoints
- [ ] 2.2 Set up the Scalar API reference with appropriate configuration options
- [ ] 2.3 Define the OpenAPI specification for the SIREN API

## 3. Integrate Scalar with Axum

- [x] 3.1 Add a new route to serve the Scalar API documentation on `/scalar`
- [x] 3.2 Ensure the Scalar API documentation uses the existing Axum router and middleware
- [x] 3.3 Configure the Scalar API documentation to use the defined OpenAPI specification

## 4. Test the Documentation

- [x] 4.1 Test that the Scalar API documentation is accessible on `/scalar`
- [x] 4.2 Verify that the documentation includes all existing endpoints
- [x] 4.3 Ensure the documentation is interactive and user-friendly
- [x] 4.4 Test the search functionality in the Scalar API documentation
- [x] 4.5 Test the example requests and responses in the Scalar API documentation

## 5. Deploy the Documentation

- [ ] 5.1 Deploy the Scalar API documentation to a staging environment
- [ ] 5.2 Monitor for any issues and address them promptly
- [ ] 5.3 Deploy to production with a rollback plan