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

# Notes about exploits
The docker CMD is rewritten so the spawned containers idle forever, and exploit
executions are run using exec with that original cmd, prepending with a
`timeout`. Therefore, do not write any complex CMDs with parenthasis, etc, and
make sure `timeout` exists on the system (or is implemented by the shell).

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

For any command you can supply `--help` to show additonal information.
```sh
$ angrepa exploit --help
Usage: angrepa exploit <command> [<args>]

manage exploits

Options:
  --help            display usage information

Commands:
  upload            upload an exploit
  download          download an exploit
  start             start an exploit
  stop              stop an exploit
  ls                list exploits
```

## Custom host
```sh
$ angrepa -h localhost:8000 ping
got pong in 34.777167ms
```

## Download template
```
$ angrepa template ls
- py_java
- python
$ angrepa template download python 
./templ_python/exploit.py
./templ_python/requirements.txt
./templ_python/Dockerfile
```

## Upload exploit
```sh
$ angrepa exploit upload templ_python --name 'template exploit' --service testservice 
Uploading 4096B file
Step 1/8 : FROM python:3.10
 ---> d9122363988f
// ... SNIP ...
Step 8/8 : CMD [ "python3", "exploit.py" ]
 ---> Running in 48596632e417
 ---> 38ce794992de
Successfully built 38ce794992de
Successfully tagged exploit_91721487919a73c2:latest
Successfully built exploit 5
```

## Start & stop
```sh
$ angrepa exploit start 5     
Started exploit 5

$ angrepa exploit ls --enabled
+----+------------------+-------------+---------+-----------+-----------+
| id | name             | service     | enabled | blacklist | pool_size |
+----+------------------+-------------+---------+-----------+-----------+
| 3  | blah             | testservice | true    |           | 1         |
+----+------------------+-------------+---------+-----------+-----------+
| 5  | template exploit | testservice | true    |           | 1         |
+----+------------------+-------------+---------+-----------+-----------+

$ angrepa exploit stop 5      
Stopped exploit 5

$ angrepa exploit ls --enabled 
+----+------------------+-------------+---------+-----------+-----------+
| id | name             | service     | enabled | blacklist | pool_size |
+----+------------------+-------------+---------+-----------+-----------+
| 3  | blah             | testservice | true    |           | 1         |
+----+------------------+-------------+---------+-----------+-----------+
```

## Download
```sh
$ angrepa exploit download 5 --path .
./download_5/exploit.py
./download_5/Dockerfile
./download_5/requirements.txt

$ angrepa exploit download 5 --path exploit_folder
exploit_folder/download_5/exploit.py
exploit_folder/download_5/Dockerfile
exploit_folder/download_5/requirements.txt

$ ls download_5 exploit_folder/download_5
download_5:
Dockerfile              exploit.py              requirements.txt

exploit_folder/download_5:
Dockerfile              exploit.py              requirements.txt
```