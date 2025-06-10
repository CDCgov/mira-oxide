# Create an argument to pull a particular version of an image
ARG rust_image
ARG rust_image=${rust_image:-rust:alpine3.21}

####################################################################################################
# BASE IMAGE
####################################################################################################
FROM ${rust_image} AS base

# Required certs for apk update
COPY ca.crt /root/ca.crt

# Put certs in /etc/ssl/certs location
RUN cat /root/ca.crt >> /etc/ssl/certs/ca-certificates.crt

# Install system libraries of general use
RUN apk update && apk add --no-cache \
    bash \
    vim \
    tar \
    dos2unix \
    build-base 

##update to rust nightly
RUN rustup toolchain install nightly

RUN rustup override set nightly

############# Copy everything into conatiner then build ##################

# Copy all scripts to docker images
COPY . .

# This build step will cache the dependencies
RUN cargo build --release

RUN rm -rf /var/lib/{apt,dpkg,cache,log}/ \
    /target/debug \
    /git* 

############# Fix vulnerablities pkgs ##################

# Convert bash script from Windows style line endings to Unix-like control characters
#RUN dos2unix ${PROJECT_DIR}/fixed_vulnerability_pkgs.sh

# Allow permission to excute the bash script
#RUN chmod a+x ${PROJECT_DIR}/fixed_vulnerability_pkgs.sh

# Execute bash script to wget the file and tar the package
#RUN bash ${PROJECT_DIR}/fixed_vulnerability_pkgs.sh  

############# Remove vulnerability pkgs ##################

# Copy all files to docker images
#COPY docker_files/remove_vulnerability_pkgs.txt ${PROJECT_DIR}/remove_vulnerability_pkgs.txt

# Copy all files to docker images
#COPY docker_files/remove_vulnerability_pkgs.sh ${PROJECT_DIR}/remove_vulnerability_pkgs.sh

# Convert bash script from Windows style line endings to Unix-like control characters
#RUN dos2unix ${PROJECT_DIR}/remove_vulnerability_pkgs.sh

# Allow permission to excute the bash script
#RUN chmod a+x ${PROJECT_DIR}/remove_vulnerability_pkgs.sh

# Execute bash script to wget the file and tar the package
#RUN bash ${PROJECT_DIR}/remove_vulnerability_pkgs.sh

############# Set up working directory ##################

# Create working directory variable
ENV WORKDIR=/data

# Set up volume directory in docker
VOLUME ${WORKDIR}

# Set up working directory in docker
WORKDIR ${WORKDIR}    

# Export project directory to PATH
ENV PATH "$PATH:/target/release"
