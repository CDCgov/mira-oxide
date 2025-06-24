# Create an argument to pull a particular version of an image

####################################################################################################
# BASE IMAGE
####################################################################################################
FROM rustlang/rust:nightly-alpine AS builder

# Required certs for apk update
COPY ca.crt /root/ca.crt

# Put certs in /etc/ssl/certs location
RUN cat /root/ca.crt >> /etc/ssl/certs/ca-certificates.crt

RUN apk update && apk add --no-cache build-base

WORKDIR /app

# Copy all scripts to docker images
COPY . .

# This build step will cache the dependencies
RUN cargo build --release

FROM alpine:latest as deploy

WORKDIR /app

COPY --from=builder \
    /app/target/release/mutations_of_interest_table \
    /app/target/release/all_sample_nt_diffs \
    /app/target/release/all_sample_hamming_dist \
    /app/target/release/plots \
    /app/target/release/check_chemistry /app/

# Create working directory variable
ENV WORKDIR=/data

# Set up volume directory in docker
VOLUME ${WORKDIR}

# Export project directory to PATH
ENV PATH "$PATH:/app"
