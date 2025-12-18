FROM alpine:3.20

RUN apk add --no-cache \
    nodejs \
    ca-certificates

# ---- Create non-root user ----
RUN adduser -D -u 10001 executor

RUN chmod 1777 /tmp

# ---- App directory (read-only at runtime) ----
WORKDIR /app
COPY target/x86_64-unknown-linux-musl/release/container_agent /app/executor

RUN chmod +x /app/executor \
    && chown executor:executor /app/executor

USER executor

EXPOSE 8000

ENTRYPOINT ["/app/executor"]
