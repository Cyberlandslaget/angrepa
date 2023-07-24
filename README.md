# angrepa
## Attack script runner for attack-defense CTFs

# Setup
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

# ...

Currently in testing phase, code is not structured correctly.