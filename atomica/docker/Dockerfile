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
COPY --chmod=755 atomica/docker/binaries/aptos-faucet-service-${GIT_SHA} /usr/local/bin/aptos-faucet-service

# Copy Move framework bundle
# The framework.mrb file contains the compiled Move modules for genesis generation
# Create the directory first, then copy with a shell to handle the glob
RUN mkdir -p /opt/aptos/framework
COPY atomica/docker/binaries/framework-*.mrb /opt/aptos/framework/
RUN mv /opt/aptos/framework/framework-*.mrb /opt/aptos/framework/head.mrb

# Verify the binaries work. aptos-node runs assert_no_test_natives before it parses its
# arguments, so `--version` also fails on a node built with Move test natives.
RUN /usr/local/bin/aptos-node --version && \
    /usr/local/bin/aptos --version && \
    /usr/local/bin/aptos-faucet-service --help

# Timelock PublicParameters blob. Built from the shipped range proof key with the aptos binary of this
# image, then checked against the committed SHA-256 and SHA3-256 (the on-chain public_parameters_hash).
# The node configs set consensus.timelock_public_parameters_blob_path to this fixed path.
COPY atomica/timelock-params/range-proof-prover-key.bin \
     atomica/timelock-params/timelock-public-parameters.sha256 \
     atomica/timelock-params/timelock-public-parameters.sha3-256 \
     /opt/aptos/timelock-params/
RUN mkdir -p /opt/aptos/etc && \
    /usr/local/bin/aptos node build-timelock-public-parameters \
        --range-proof-key /opt/aptos/timelock-params/range-proof-prover-key.bin \
        --output /opt/aptos/etc/timelock-public-parameters.bcs && \
    cd /opt/aptos/etc && sha256sum -c /opt/aptos/timelock-params/timelock-public-parameters.sha256 && \
    /usr/local/bin/aptos node validate-timelock-public-parameters \
        /opt/aptos/etc/timelock-public-parameters.bcs \
        --expected-hash "$(cat /opt/aptos/timelock-params/timelock-public-parameters.sha3-256)"

# Create necessary directories and set permissions
RUN mkdir -p /opt/aptos/var /opt/aptos/data /opt/aptos/etc && \
    chown -R aptos:aptos /opt/aptos

# Add build metadata
# The node is the release build: no `testing` cargo feature. aptos-node refuses to compile
# with it (compile_error in aptos-node/src/lib.rs), and the check above rejects test natives.
LABEL io.atomica.aptos-node.cargo-features="default" \
      io.atomica.aptos-node.test-natives="false"
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
    CMD curl -f http://localhost:8080/v1/-/healthy || exit 1

# Use Tini as entrypoint for proper signal handling
ENTRYPOINT ["/tini", "--", "/usr/local/bin/aptos-node"]
CMD ["-f", "/opt/aptos/validator.yaml"]
