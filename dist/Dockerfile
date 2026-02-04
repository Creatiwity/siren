FROM gcr.io/distroless/static-debian13
COPY --chmod=+x ./sirene /app/
WORKDIR /app
ENV HOST=0.0.0.0
ENV PORT=3000
EXPOSE 3000
ENTRYPOINT ["./sirene", "serve"]
