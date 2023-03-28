# discoenv-oxidize

A set of services for the Discovery Environment written in Rust.

## Tooling
You're going to need the following tools to work with the services in this repository.
* Rust - The Rust programming language.
* Cargo - Multi-tool for the Rust programming language.
* sqlx CLI - A command-line tool related to the SQLX crate.
* protoc - Protocol Buffer compiler.
* Docker - Needed for building container images.
* kubectl - Needed for deployments.
* protoc - Needed for builds.
* Skaffold - Needed for deployments.

If you're just interested in dealing with the code, then you only need Rust, Cargo, and protoc.

### Rust & Cargo
You're going to be using Rust and Cargo a lot if you're developing and/or building the projects in this repo, so make sure you have them installed and in your $PATH.

We're using Rust and Cargo `1.68.1` at the time of this writing.

To install Rust and Cargo for your development environment, go to [rust-lang.org](https://www.rust-lang.org/tools/install) and follow their instructions.

### sqlx CLI
The sqlx library macros `query!` and `query_as!` will both use a connection to a database configured with the `DATABASE_URI` environment variable. You can turn this off either by unsetting that environment variable or by setting the environment variable `SQLX_OFFLINE=true`.

Connecting to a database during the CI/CD process would be a problem, so you'll need to install the sqlx CLI and use it to serialize query information that can be used during compilation. Documentation for installing the sqlx CLI is available on [github.com](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md).

To serialize the queries, do the following:
```bash
cargo sqlx prepare --merged
```

NOTE: The sqlx docs say to use the `--workspace` flag instead of `--merged`, but the tool itself said to use `--merged`. YMMV.

### protoc
protoc is the protocol buffer compiler. You're going to need to have it installed and in your path in order to build all of the dependencies for the project.

We're using protoc `3.21.11` at the time of this writing.

For this repository, you just need the `protoc` compiler, the plugins needed by the [p](https://github.com/cyverse-de/p) repository are not a requirement.

See the installation instructions [here](https://grpc.io/docs/protoc-installation/) for more information.

### Docker
Docker is mainly used to prepare the services for deployment. You don't necessarily need to have it installed if you're just writing code, but if you want to build and deploy the services into a Kubernetes cluster, then you're going to want it (or Buildah).

Just use a reasonably up to date version of Docker. If you're on MacOS, use Docker Desktop.

See [docker.com](https://www.docker.com/) for more information.

### Skaffold
Skaffold is a tool for building the container images and deploying them into the cluster. If you're not involved with building or deploying the container images, then you don't need it.

Use skaffold version `2.2.0` or later.

For more information on installing Skaffold, see [skaffold.dev](https://skaffold.dev/docs/install/#standalone-binary).

### Sources
* [rust-lang.org](https://www.rust-lang.org/tools/install)
* [grpc.io](https://grpc.io/docs/protoc-installation/)
* [docker.com](https://www.docker.com/)
* [skaffold.dev](https://skaffold.dev/docs/install/#standalone-binary)

---

## Repository Organization
You're really going to want to read the [Package Layout](https://doc.rust-lang.org/cargo/guide/project-layout.html) and [Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html) sections of the Cargo Book to understand what is going on in this repo.

Specifically, the top-level directory is a virtual workspace and there are multiple packages defined in the workspace.

The main goals of this repo's organization are as follows (in no particular order):
* Safely consolidate as many of the microservices as possible.
* Provide a common set of modules to make creating new microservices fairly easy.
* Unify domain object representations across service boundaries.
* Move as much code into a single repository as possible.
* Reduce the number of container images needed to deploy a full set of services.
* Still provide enough flexibility to allow for services that deviate from the common set of libraries and practices.

### Directory Structure
* `Cargo.toml` - The workspace's Cargo.toml file. Lists which directories are members of the workspace.
* `skaffold.yaml` - The skaffold YAML file for building and deploying container images into k8s.
* `discoenv/` - The discoenv crate containing the services and shared libraries deployed from this repo.
* `service_errors/` - A crate containing common error handling code that integrates with the ServiceError type defined by protocol buffer in the `p` repo.
* `service_signals/` - A crate containing signal processing code that each service should use.
* `db/` - A crate providing access to data in the Discovery Environment databases. Uses SQLX.
* `k8s/` - Container image build and deployment information.

### Workspace
The top-level directory of the repository is a Cargo workspace. This allows us to provide multiple Cargo crates from a single repository. The intention is not for every microservice to have its own crate; instead, services should go into the `discoenv` crate inside the workspace. If a service absolutely needs to exist outside of the discoenv crate, then it can still reside inside this workspace as a new crate.

### Discoenv Crate
The `discoenv` crate is where you should put new microservice code by default. If it's relatively simplistic code that provides access to information in the database as JSON, then consider just adding the functionality to the default binary, `discoenv`. If the code is a bit more complicated and would benefit from being able to scale separately from the rest of the services, then put it into a secondary binary in the `discoenv` crate.

The primary binary for the `discoenv` crate is defined in `discoenv/src/main.rs` and is a service that provides access to relatively simple HTTP/JSON code that access the database and return JSON encoded information with minimal processing.

The `user-info` service provides access to `bags`, `sessions`, `preferences`, and `saved-searches` defined by users. It is in the `discoenv/src/bin/user-info/main.rs` file.

The `discoenv/src/lib.rs` file exposes modules that are provided by the `discoenv` library. They can be reused across binaries (a.k.a services) contained within the `discoenv` crate.

### Sources
* [Package Layout](https://doc.rust-lang.org/cargo/guide/project-layout.html)
* [Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)

---

## Building and Deploying
There are two things that need to be built from this repository: binaries and container images. During development, you'll be building binaries pretty regularly. Container images are built if and when you're ready to deploy into a cluster.

### Cargo
Cargo is provided by `rustup` and is used to build the Rust services. From the top-level directory run the following to build everything:
```bash
cargo build --workspace
```

If you want to build release versions of the binaries, do the following:

```bash
cargo build --workspace --release
```

The binaries will land in the top-level `target` directory. That directory should not be checked in.

### Docker
If you want to build the container images locally, there are two images to build: `rust-builder` and `disconev-oxidize`. 

`rust-builder` is used to build and cache the dependencies so you don't have to do a full rebuild every time.

`discoenv-oxidize` contains all of the microservice binaries. The Dockerfile for it is multi-stage. 

Hopefully you won't need to do this manually locally unless you're working on the images.

First, build rust-builder:
```bash
docker build -t harbor.cyverse.org/de/rust-builder .
```

Then build the image:
```bash
docker build -t harbor.cyverse.org/de/discoenv-oxidize .
```

### Skaffold
Skaffold is used to automate building and deploying the images. The k8s resource manifests are located in the `k8s` directory.

Do not run these commands locally if your target environment's processor architecture is different from your local architecture. In other words, don't run this on an M1 or M2 Mac if you're deploying on an x86_64 Linux box.

Build and push the images.
```bash
skaffold build --file-output build.json
```

Deploy the built images.
```bash
skaffold deploy -a build.json
```
