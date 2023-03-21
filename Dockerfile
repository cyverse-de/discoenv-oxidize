FROM rust:1.68 as build-env
## One of the dependencies needs protoc installed to compile.
RUN apt-get update -y && apt-get install -y protobuf-compiler
WORKDIR /usr/src/discoenv-oxidize
COPY . .
RUN cargo build --workspace --release

FROM gcr.io/distroless/cc
COPY --from=build-env  /usr/src/discoenv-oxidize/target/release/user-info /user-info

