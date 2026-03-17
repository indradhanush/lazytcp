# lazytcp

[![Coverage](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/indradhanush/97cfd90277d7142cfe8dc5f56e952172/raw/75079a418653e2e879f26b89bd543e97192bb744/lazytcp.json?v=1)](https://github.com/indradhanush/lazytcp/actions/workflows/coverage.yml)

`lazytcp` is a TUI for interactively filtering packets from a `.pcap` file. It aims to provide a clean, fast and intuitive UX.
Hat tip to the awesome [lazygit](https://github.com/jesseduffield/lazygit) TUI for the name inspiration. 🙌


## Requirements

- [Rust](https://rust-lang.org/tools/install/)
- `tcpdump` available on `PATH`

## Installation

```bash
git clone git@github.com:indradhanush/lazytcp.git
cd lazytcp
cargo install --path . 
```

## Keyboard Controls

- `q` / `Ctrl-C`: quit
- `tab` / `shift+tab`: cycle focus between panes
- `0`: focus Filter pane
- `1`: focus Packets pane
- `j` / `k` or arrow keys: move selection
- `?`: open keybindings popup
- `C`: clear all active filters

### Filter pane

- - `j` / `k` or arrow keys: move selection
- `enter`: open value popup for the selected filter
- `c`: clear selected filter dimension

#### Value popup (multi-select)

- `space`: toggle selected value
- `enter`: apply
- `/`: start sub string search and type to narrow down candidates, `enter` to finish typing
- `c`: clear current category
- `esc`: cancel

#### Date Time popup

- `tab` or `j`/`k`: switch start/end field
- type start/end timestamps directly
- `c`: clear both fields
- `C`: clear all categories
- `enter`: apply
- `esc`: cancel

## Notes

- Current CLI usage is: `lazytcp <pcap-file>`.
- Packet parsing is based on `tcpdump -nn -tttt -r <pcap-file>`.

## Development

See [DEVELOPMENT.md](./DEVELOPMENT.md)

## Roadmap

See [ROADMAP.md](docs/roadmap.md). 
