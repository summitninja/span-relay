# Use a Rust base image for building
FROM rust:alpine as builder

WORKDIR /app

# Update packages
RUN apk update && \
    apk upgrade

# Add musl dev
RUN apk add --no-cache musl-dev

RUN update-ca-certificates

# Create appuser
ENV USER=span_relay
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"


# Copy the project files to the container
COPY . .

# Build to root dir for consistant copy
RUN CARGO_TARGET_DIR=/app/span_relay cargo build --release


# Use alpine as the final base image
FROM scratch

# Import from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

# Set the working directory
WORKDIR /app

# Copy the built binary from the builder stage
COPY --from=builder /app/span_relay .

USER span_relay:span_relay

# Set the binary as the entrypoint
ENTRYPOINT ["/app/my_app"]
