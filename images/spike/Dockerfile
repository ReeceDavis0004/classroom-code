FROM rust:1.94.1-alpine3.23 AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p runner

FROM alpine:3.23
COPY --from=build /src/target/release/runner /judge/runner
