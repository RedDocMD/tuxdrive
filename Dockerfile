FROM rust:1-alpine3.14

RUN apk update && apk add crystal shards musl-dev

RUN mkdir -p /code /cargodir
VOLUME ["/code", "/cargodir"]

WORKDIR /code
ENV CARGO_HOME=/cargodir