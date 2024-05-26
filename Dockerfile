# Use a Rust base image for both build and runtime stages to ensure compatibility
FROM rust:latest as builder
WORKDIR /usr/src/app
COPY . .
RUN cargo install --path .

# Use the same base image for the runtime to ensure all required libraries are available
FROM rust:latest
WORKDIR /usr/src/app
COPY --from=builder /usr/local/cargo/bin/crewai_server /usr/local/bin/crewai_server
CMD ["crewai_server"]
