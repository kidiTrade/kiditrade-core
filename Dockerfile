FROM rust:latest as build
WORKDIR /app

RUN rustup target add x86_64-unknown-linux-musl

COPY . ./

RUN cargo build --target  x86_64-unknown-linux-musl
