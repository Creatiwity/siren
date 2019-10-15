FROM clux/muslrust:nightly AS builder
RUN rustc --version && cargo --version
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations/
COPY src ./src/
RUN cargo build --release --target x86_64-unknown-linux-musl
RUN strip target/x86_64-unknown-linux-musl/release/sirene
RUN cp target/x86_64-unknown-linux-musl/release/sirene /

FROM alpine
WORKDIR /app
COPY --from=builder /sirene /app/
ENV HOST 0.0.0.0
ENV PORT 3000
EXPOSE 3000
CMD ["/bin/sh", "-c", "./sirene serve"]
