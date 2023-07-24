# angrepa
## Attack script runner for attack-defense CTFs

# Setup
0. Tooling
```
cargo install diesel_cli --no-default-features --features postgres
```

1. Start DB and adminer
```
docker compose up
```
    - Optional: change username/password in config and docker-compose.yml

2. Setup DB
```
diesel setup
diesel migration run
```

3. Create a [config](./config/)

4. Create a CTF specific [fetcher](./src/manager/fetcher/) and [submitter](./src/manager/submitter/).

5. Run with the specific config
```
cargo r -- config/CONFIG.toml
```
or
```
cargo build --release
./target/release/angrepa config/CONFIG.toml
```

# ...

Currently in testing phase, code is not structured correctly.