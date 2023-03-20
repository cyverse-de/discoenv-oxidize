FROM rust-builder as build-env
WORKDIR /usr/src/discoenv-oxidize
COPY . .
RUN cargo build --workspace --release

FROM gcr.io/distroless/cc
COPY --from=build-env  /usr/src/discoenv-oxidize/target/release/user-info /user-info

