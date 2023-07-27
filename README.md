# angrepa
## Attack script runner for attack-defense CTFs

# Client (CLI) Setup
To install the `angrepa` cli, you need an up to date rust installation. `rustup` is strongly recommended, and a one-liner for your operating system can be found at https://rustup.rs/

1. Downloading the repo
```
git clone git@github.com:cyberlandslaget/angrepa.git
cd angrepa
```

2. Installing angrepa
```
cargo install --path . --bin angrepa
```

3. Updating angrepa
```
git pull
cargo install --path . --bin angrepa
```

Angrepa does not update itself, so make sure to update it! In the future it's
planned that include the git hash in the client and server, and give a warning
if the client is outdated.

See [CLI Examples](#cli-by-example) for some usage examples.

# Server Setup
0. Tooling
```
cargo install diesel_cli --no-default-features --features postgres
```

1. Create a [config](./config/)

2. Create a CTF specific [fetcher](./src/manager/fetcher/) and [submitter](./src/manager/submitter/)
    - Add your fetcher and submitter to [fetcher.rs](./src/manager/fetcher.rs) and [submitter.rs](./src/manager/submitter.rs)

3. Start DB and adminer
    - Optional: change username/password in [config](./config/) and [docker-compose.yml](./docker-compose.yml)
```
docker compose up
```

4. Setup DB
```
diesel setup
diesel migration run
```

5. Run with the specific config
```
cargo r -- config/CONFIG.toml
```
or
```
cargo build --release
./target/release/angrepa config/CONFIG.toml
```

## Debugging
angrepa uses `tracing` for logging, so you can set the `RUST_LOG` environment
variable to enable more logging. additionally the `--debug` flag automatically
sets the equivalent of `RUST_LOG=debug,hyper=info`.
```sh
$ cargo r config/enowars7.toml --debug
2023-...Z DEBUG angrepa::config: Start time: 2023-07-22T12:00:00Z
2023-...Z DEBUG angrepa::config: Current time: 2023-07-25T13:18:39.054039Z
2023-...Z  INFO angrepa::runner: Manager woke up!
^C
$ RUST_LOG=trace cargo r config/enowars7.toml
2023-...Z DEBUG hyper::client::connect::http: connected to 23.88.111.63:443
2023-...Z  INFO angrepa::manager::handler: Got 0 results, 0 accepted.
2023-...Z TRACE hyper::client::conn: client handshake Http1
2023-...Z TRACE hyper::client::client: handshake complete, spawning background dispatcher task
```

# CLI by Example
The default host is `http://angrepa.cybl`. http is assumed if no prefix is
specified.

## Custom host
```sh
$ angrepa -h localhost:8000 ping
got pong in 34.777167ms
```

## Download template
```
$ angrepa template ls
- python
- py_java
$ angrepa template download python
./templ_python/exploit.py
./templ_python/Dockerfile
./templ_python/requirements.txt
```

## Upload
```sh
$ angrepa upload templ_python --name 'my exploit' --service 'testservice'
Uploading 4096B file
Sucessfully built exploit 2
```

## Start & stop
```sh
$ angrepa start 2
Started exploit 2
$ angrepa stop 2
Stopped exploit 2
```

## Download
```
$ angrepa download 2 --path .
./download_2
./download_2/exploit.py
./download_2/Dockerfile
./download_2/requirements.txt
```