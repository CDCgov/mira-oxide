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

# May only be required for WSL.
# Required certs for apk update
COPY ca.crt /root/ca.crt

# Put certs in /etc/ssl/certs location
RUN cat /root/ca.crt >> /etc/ssl/certs/ca-certificates.crt

# Install system libraries of general use
RUN apk update && apk add --no-cache \
    bash \
    && rm -rf /var/lib/{apt,dpkg,cache,log} \
    && rm /root/ca.crt

WORKDIR /app

COPY --from=builder \
    /app/target/release/mutations_of_interest_table \
    /app/target/release/all_sample_nt_diffs \
    /app/target/release/all_sample_hamming_dist \
    /app/target/release/plots /app/


# Create working directory variable
ENV WORKDIR=/data

# Set up volume directory in docker
VOLUME ${WORKDIR}

# Set up working directory in docker
WORKDIR ${WORKDIR}    

# Export project directory to PATH
ENV PATH "$PATH:/app"
