# clean
clean:
    cargo clean

check-with-tests:
    cargo check --workspace --tests

build-with-tests:
    cargo build --workspace --tests

build:
    cargo build --workspace

udeps:
    cargo +nightly udeps --workspace

build-chatty-types:
    cd chatty-types && \
    cargo build --package chatty-types

build-chatty-tcp:
    cd chatty-tcp && \
    cargo build --package chatty-tcp

# watching builds
watch-build:
    cargo watch -x "build --workspace"

watch-build-with-tests:
    cargo watch -x "build --workspace --tests"

watch-build-chatty-types:
    cd chatty-types && \
    cargo watch -x "build --package chatty-types"

watch-build-chatty-tcp:
    cd chatty-tcp && \
    cargo watch -x "build --package chatty-tcp"

# .cargo/cargo.toml has TCP_SERVER_ADDRESS AND TCP_SERVER_PORT and we can overrride it here as needed if needed
# run only

# server
run-chatty-tcp-server:
    TCP_SERVER_ADDRESS="localhost" TCP_SERVER_PORT="8081" cargo run --package chatty-tcp --bin server

run-chatty-tcp-client-cmd-help:
    cargo run --package chatty-tcp --bin client -- --help

run-chatty-tcp-client-join-no-username:
    TCP_SERVER_ADDRESS="localhost" TCP_SERVER_PORT="8081" cargo run --quiet --package chatty-tcp --bin client

run-chatty-tcp-client-carl-join:
    TCP_SERVER_ADDRESS="localhost" TCP_SERVER_PORT="8081" cargo run --quiet --package chatty-tcp --bin client -- --username "carl"

run-chatty-tcp-client-david-join:
    TCP_SERVER_ADDRESS="localhost" TCP_SERVER_PORT="8081" cargo run --quiet --package chatty-tcp --bin client -- --username "david"

run-chatty-tcp-client-lucio-join:
    TCP_SERVER_ADDRESS="localhost" TCP_SERVER_PORT="8081" cargo run --quiet --package chatty-tcp --bin client -- --username "lucio"

# .cargo/cargo.toml has SEND_CAL_EVENTS = "true" so we can overrride it here as needed

# watch running
watch-chatty-tcp-server:
    cargo watch --why --poll  -x "run --package chatty-tcp --bin server"

watch-run-chatty-tcp-client-carl-join:
    cargo watch -x "run --package chatty-tcp --bin client -- --username \"carl\""

watch-run-chatty-tcp-client-david-join:
    cargo watch -x "run --package chatty-tcp --bin client -- --username \"david\""

watch-run-chatty-tcp-client-lucio-join:
    cargo watch -x "run --package chatty-tcp --bin client -- --username \"lucio\""

# tests
test:
    cargo test --workspace --tests -- --show-output

test-chatty-tcp:
    cargo test --package chatty-tcp --tests -- --show-output

# watch test
watch-test:
    cargo watch -x "test --workspace"
