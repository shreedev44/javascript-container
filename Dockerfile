FROM debian:trixie-slim

# Avoid interactive prompts
ENV DEBIAN_FRONTEND=noninteractive

# Install system dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    nodejs \
    npm \
    bubblewrap \
    procps \
    iproute2 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user (recommended)
RUN useradd -m executor

# Set workdir
WORKDIR /app

# Copy Rust binary (assumes you build it locally)
COPY target/release/container_agent /app/executor

# Permissions
RUN chown -R executor:executor /app

# Expose port used by executor
EXPOSE 8000

# Start executor
ENTRYPOINT ["/app/executor"]
