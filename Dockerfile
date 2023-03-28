# Prepare the list of dependencies to build.
FROM harbor.cyverse.org/de/rust-builder as planner
WORKDIR /usr/src/discoenv-oxidize
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Build and cache the dependencies
FROM planner as builder
COPY --from=planner /usr/src/discoenv-oxidize/recipe.json .
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --workspace --release

FROM gcr.io/distroless/cc
COPY --from=builder  /usr/src/discoenv-oxidize/target/release/discoenv /discoenv

