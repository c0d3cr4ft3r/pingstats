# 🛰️ pingstats

**pingstats** is a Rust-powered network diagnostics tool that combines multiple utilities—`ping`, `traceroute`, CSV export, and a TUI visualizer—into a single modern CLI app.

---

## 🔧 Features

| Command      | Description                                                                       |
| ------------ | --------------------------------------------------------------------------------- |
| `tui`        | Live terminal UI with RTT chart and stats. Quit with `q`.                         |
| `csv`        | Records latency data to a CSV file with timestamps.                               |
| `compare`    | Pings multiple hosts and summarizes response time & packet loss side-by-side.     |
| `traceroute` | ICMP-based traceroute (limited to final hop on macOS due to socket restrictions). |

---

## 🧪 Example Usage

```bash
# Live RTT chart for google.com
sudo pingstats tui -H google.com

# Export latency to CSV (50 pings, 1s interval)
sudo pingstats csv -H 1.1.1.1 --count 50 --interval 1.0 --output logs/ping.csv

# Compare multiple hosts
sudo pingstats compare --hosts 1.1.1.1 --hosts 8.8.8.8 --hosts cloudflare.com

# Traceroute (up to 20 hops)
sudo pingstats traceroute -H wikipedia.org --max-hops 20
```

---

## 📦 Install

Make sure you have **Rust 1.87+** and admin rights (required for raw ICMP sockets).

```bash
git clone https://github.com/c0d3cr4ft3r/pingstats.git
cd pingstats
cargo build --release
```

Or install globally:

```bash
cargo install --path .
```

---

## ⚠ Notes

- Requires **sudo** or administrator access to open raw ICMP sockets.
- On macOS, `traceroute` only shows the final hop due to TTL limitations.
- Uses `tokio`, `pnet`, `ratatui`, and `crossterm` under the hood.

---

## 📁 Project Structure

```
src/
├── main.rs         # CLI entrypoint with subcommands
├── tui.rs          # Terminal UI module (graph, live stats)
├── csv.rs          # CSV export of ping results
├── compare.rs      # Compare hosts RTT and loss
├── traceroute.rs   # ICMP traceroute (simple)
```

---

## 🧠 Why This Exists

To give developers and sysadmins a clean, fast, async-first tool for network diagnostics with a beautiful terminal UI and extensible subcommands.

---

## 📝 License

MIT
