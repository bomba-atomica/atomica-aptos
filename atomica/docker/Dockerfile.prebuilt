# Atomica Aptos Validator Node - Prebuilt Binary
#
# This Dockerfile uses prebuilt aptos binaries downloaded by GitHub Actions
# instead of building from source, reducing build time and resource usage.
#
# Build context: repository root
#
# Build args:
#   BUILD_DATE: Build timestamp
#   GIT_SHA: Git commit hash
#   BINARY_RELEASE_TAG: GitHub release tag for the binary (e.g., binary-abc1234)

FROM ubuntu:24.04

ARG TARGETARCH=amd64
ARG GIT_SHA
ARG BINARY_RELEASE_TAG

# Validate required build args
RUN if [ -z "$GIT_SHA" ]; then \
        echo "Error: GIT_SHA build arg is required"; \
        exit 1; \
    fi && \
    if [ -z "$BINARY_RELEASE_TAG" ]; then \
        echo "Error: BINARY_RELEASE_TAG build arg is required"; \
        exit 1; \
    fi

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    iproute2 \
    libpq5 \
    libssl3t64 \
    libdw1t64 \
    && rm -rf /var/lib/apt/lists/*

# Add Tini to make sure the binaries receive proper SIGTERM signals when Docker is shut down
ADD --chmod=755 https://github.com/krallin/tini/releases/download/v0.19.0/tini-${TARGETARCH} /tini

# Create aptos user
RUN groupadd -g 6180 aptos && \
    useradd -u 6180 -g aptos -s /bin/bash -m aptos

# Copy prebuilt binaries (downloaded by GitHub Actions)
# These binaries are downloaded and verified in the CI workflow before building the image
COPY --chmod=755 atomica/docker/binaries/aptos-node-${GIT_SHA} /usr/local/bin/aptos-node
COPY --chmod=755 atomica/docker/binaries/aptos-${GIT_SHA} /usr/local/bin/aptos

# Verify the binaries work
RUN /usr/local/bin/aptos-node --version && \
    /usr/local/bin/aptos --version

# Create data directory
RUN mkdir -p /opt/aptos/var && \
    chown -R aptos:aptos /opt/aptos

# Add build metadata
ARG BUILD_DATE
ENV BUILD_DATE=${BUILD_DATE}
ENV GIT_SHA=${GIT_SHA}
ENV BINARY_RELEASE_TAG=${BINARY_RELEASE_TAG}

# Expose ports
EXPOSE 6180 6181 6182 8080 9101

# Set working directory
WORKDIR /opt/aptos

# Run as aptos user
USER aptos

# Health check
HEALTHCHECK --interval=10s --timeout=5s --retries=3 \
    CMD curl -f http://localhost:8080/v1 || exit 1

# Use Tini as entrypoint for proper signal handling
ENTRYPOINT ["/tini", "--", "/usr/local/bin/aptos-node"]
CMD ["-f", "/opt/aptos/validator.yaml"]
