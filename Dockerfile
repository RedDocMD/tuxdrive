FROM rust:1-alpine3.14

RUN apk update && apk add crystal shards musl-dev bash

RUN mkdir -p /code /cargodir /testdir /target
VOLUME ["/code", "/cargodir"]

WORKDIR /code
ENV CARGO_HOME=/cargodir
ENV CARGO_TARGET_DIR=/target