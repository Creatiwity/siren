FROM alpine
WORKDIR /app
COPY ./sirene /app/
ENV HOST 0.0.0.0
ENV PORT 3000
EXPOSE 3000
CMD ["/bin/sh", "-c", "./sirene serve"]
