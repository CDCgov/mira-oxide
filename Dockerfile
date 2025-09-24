# Create an argument to pull a particular version of an image

####################################################################################################
# BASE IMAGE
####################################################################################################
FROM rustlang/rust:nightly-alpine AS builder

# Replace with other certs if needed
COPY .certs/min-cdc-bundle-ca.crt /etc/ssl/certs/ca.crt

# Required certs for gitlab and cargo
RUN cat /etc/ssl/certs/ca.crt >> /etc/ssl/certs/ca-certificates.crt

# Install required packages, including bash
RUN apk update && apk add --no-cache \
    build-base \
    openssl-dev \
    bash

# Set workdir
WORKDIR /app

# Copy all scripts and source code to the Docker image
COPY . .

# This build step will cache the dependencies
RUN cargo build --release \
    || CARGO_HTTP_CAINFO=/etc/ssl/certs/ca.crt cargo build --release

####################################################################################################
# DEPLOY IMAGE
####################################################################################################
FROM alpine:3.18

# Required certs for apk update
COPY .certs/min-cdc-bundle-ca.crt /etc/ssl/certs/ca.crt

# Put certs in /etc/ssl/certs location
RUN cat /etc/ssl/certs/ca.crt >> /etc/ssl/certs/ca-certificates.crt

RUN apk update && apk add --no-cache bash

# Set workdir
WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/mira-oxide /app/

# Create working directory variable
ENV WORKDIR=/data

# Set up volume directory in Docker
VOLUME ${WORKDIR}

# Export project directory to PATH
ENV PATH="$PATH:/app"

# Set bash as the default shell
SHELL ["/bin/bash", "-c"]

# Default command to run when the container starts
CMD ["bash"]
