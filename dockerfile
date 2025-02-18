# Jackson Coxson
# todo finish this

# Use a base image with Rust for building the project
FROM rust:latest AS builder

# Install Python and pip (required for tunneld dependencies)
RUN apt-get update && apt-get install -y \
    python3 \
    python3-pip \
    wireguard-tools && \
    rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Clone and build netmuxd
RUN git clone https://github.com/jkcoxson/netmuxd.git && \
    cd netmuxd && \
    git reset --hard 6b4941dbed8fac38c67db031be0309717ff6b4e3 && \
    cargo build --release && \
    cd ..

RUN git clone https://github.com/jkcoxson/tunneld-rs.git && \
    cd tunneld-rs && \
    git reset --hard 7371cffbf93f4984e9c54186a29e1c6dd6775eb1 && \
    cargo build --release && \
    cd ..

# Copy the project files into the container
COPY . .

# Build the JitStreamer EB project in release mode
RUN cargo build --release

# Prepare the final runtime image
FROM debian:bookworm-slim

# Install required runtime dependencies
RUN apt-get update && apt-get install -y \
    python3 \
    python3-pip \
    wireguard-tools \
    iproute2 \
    librust-openssl-dev \
    libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy the built binary and necessary files from the builder stage
COPY --from=builder /app/target/release/jitstreamer-eb /usr/local/bin/jitstreamer-eb
COPY --from=builder /app/netmuxd/target/release/netmuxd /usr/local/bin/netmuxd
COPY --from=builder /app/tunneld-rs/target/release/tunneld-rs /usr/local/bin/tunneld-rs
COPY --from=builder /app/requirements.txt /app/requirements.txt
COPY --from=builder /app/src/runners /app/src/runners

# Ensure src/runners/*.python files are included
COPY src/runners /app/src/runners

# Install Python dependencies for tunneld
# Break the system packages, I drink error message tears for breakfast
RUN pip3 install -r /app/requirements.txt --break-system-packages

# Set the default working directory
WORKDIR /app
RUN mkdir -p /var/lib/lockdown
RUN mkdir -p /etc/wireguard

# Expose Wireguard and Jitstreamer ports
EXPOSE 51869/udp
EXPOSE 9172/tcp

VOLUME /var/lib/lockdown
VOLUME /etc/wireguard
VOLUME /app/jitstreamer.db

# Command to start all required services and run the program
CMD ["/bin/bash", "-c", "wg-quick up jitstreamer & netmuxd & tunneld-rs & jitstreamer-eb"]
