# Development


## Configure PATH to use lazytcp anywhere

- Update `PATH_TO_LAZYTCP` and add the following to your bashrc or equivalent:
```bash
export PATH=$PATH:<PATH_TO_LAZYTCP>/target/debug
```

```bash
cargo build
```

## Run

```bash
cargo run -- <pcap-file>
```

Example:

```bash
cargo run -- capture.pcap
```

CLI help:

```bash
cargo run -- --help
```


