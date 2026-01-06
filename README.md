# ⚡Async Port Scanner

A blazing-fast, concurrent port scanner written in Rust using async/await and Tokio. Capable of scanning all 65,535 ports in just seconds!

## 🚀 Features

- **Extreme Performance**: Scans 65k ports in ~3-8 seconds
- **Massive Concurrency**: Run up to 15,000+ concurrent tasks
- **Async/Await**: Built with Tokio for efficient I/O operations
- **Flexible Configuration**: Customize timeout, concurrency, and port ranges
- **Service Detection**: Identifies common services on open ports
- **User-Friendly CLI**: Simple command-line interface with shortcuts

## 📋 Prerequisites

- Rust 1.70 or higher
- Cargo (comes with Rust)

## 🔧 Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/port-scanner.git
cd port-scanner

# Build the project
cargo build --release
```

## 🎯 Usage

### Basic Usage

```bash
# Scan specific port range
cargo run -- 127.0.0.1 1 1000

# Scan all ports (1-65535)
cargo run -- 192.168.1.1 all

# Scan common ports (1-1000)
cargo run -- scanme.nmap.org common
```

### Advanced Usage

```bash
# Custom timeout and concurrency
cargo run -- 192.168.1.1 1 65535 100 15000
#                                  |   |
#                                  |   └─ Concurrent tasks
#                                  └───── Timeout in ms

# Fast mode preset (50ms timeout, 15000 concurrent)
cargo run -- 192.168.1.1 all fast
```

### Command Format

```
cargo run -- <target_ip> <start_port> <end_port> [timeout_ms] [concurrent_limit]
```

**Arguments:**
- `target_ip`: IP address or hostname to scan
- `start_port`: First port to scan (1-65535)
- `end_port`: Last port to scan (1-65535)
- `timeout_ms`: (Optional) Connection timeout in milliseconds (default: 50)
- `concurrent_limit`: (Optional) Number of concurrent tasks (default: 10000)

**Shortcuts:**
- `all`: Scan all ports 1-65535
- `common`: Scan common ports 1-1000
- `fast`: Use aggressive settings (50ms timeout, 15000 concurrent)

## 📊 Performance

Scanning all 65,535 ports on localhost:

| Configuration | Time | Ports/sec |
|--------------|------|-----------|
| Default (10k concurrent, 50ms) | ~5-8s | ~8,000-13,000 |
| Fast mode (15k concurrent, 50ms) | ~3-5s | ~13,000-21,000 |
| Extreme (20k concurrent, 30ms) | ~2-3s | ~21,000-32,000 |

*Performance varies based on network conditions, target responsiveness, and system resources.*

## 🛠️ How It Works

This scanner uses Rust's async/await with Tokio runtime for efficient concurrent I/O:

1. **FuturesUnordered**: Manages thousands of concurrent tasks
2. **Streaming Pattern**: Continuously refills task queue as connections complete
3. **Non-blocking I/O**: Uses async TCP connections instead of threads
4. **Optimal Resource Usage**: Minimal memory footprint compared to thread-based scanners

### Why It's Fast

- **No batching delays**: Tasks are added immediately as others complete
- **Lightweight tasks**: Async tasks use ~2KB vs ~2MB for threads
- **Efficient I/O**: Tokio's event loop handles thousands of connections on few threads
- **Smart timeouts**: Aggressive timeouts prevent hanging on closed ports

## ⚙️ Configuration

### Adjusting Speed vs Accuracy

**For maximum speed:**
```bash
cargo run -- <target> all 30 20000
# Very fast but may miss slow-responding ports
```

**For maximum accuracy:**
```bash
cargo run -- <target> all 500 5000
# Slower but catches all open ports reliably
```

**Balanced (recommended):**
```bash
cargo run -- <target> all 100 10000
# Good balance of speed and accuracy
```

## 🔍 Service Detection

The scanner identifies common services on open ports:

- Port 21: FTP
- Port 22: SSH
- Port 23: Telnet
- Port 25: SMTP
- Port 53: DNS
- Port 80: HTTP
- Port 443: HTTPS
- Port 3306: MySQL
- Port 3389: RDP
- Port 5432: PostgreSQL
- And more...

## ⚠️ Warnings

- **High concurrency** (>10,000) may trigger rate limits or firewalls
- **Aggressive timeouts** (<50ms) may miss slow-responding services
- **Network impact**: Scanning generates significant network traffic
- **Legal considerations**: Only scan networks you own or have permission to test

## 🐛 Troubleshooting

### "Too many open files" error
Increase your system's file descriptor limit:
```bash
ulimit -n 65535
```

### Scan seems slow
- Check network latency to target
- Increase concurrent limit
- Decrease timeout
- Ensure no firewall is rate-limiting you

### Missing open ports
- Increase timeout (some services respond slowly)
- Reduce concurrent limit (may overwhelm target)
- Check if target has rate limiting

## 📝 Example Output

```
---- Async Port Scanner ----

🎯 Target:          192.168.1.1
📊 Port range:      1-65535
🔢 Total ports:     65535
⚡ Concurrent:      10000 tasks
⏱️  Timeout:         50ms per port

🚀 Starting scan...

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ Found 5 open port(s):

  Port    22 - SSH
  Port    80 - HTTP
  Port   443 - HTTPS
  Port  3306 - MySQL
  Port  8080 - HTTP-Alt
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
⏱️  Scan completed in 4.73s
🚀 Scanned 65535 ports at 13,851 ports/sec

```

## 🤝 Contributing

Contributions are welcome! Feel free to:

- Report bugs
- Suggest features
- Submit pull requests
- Improve documentation

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## ⚖️ Legal Disclaimer

This tool is provided for educational and ethical testing purposes only. Users are responsible for complying with applicable laws and regulations. Only scan networks and systems you own or have explicit permission to test. Unauthorized port scanning may be illegal in your jurisdiction.

## 🙏 Acknowledgments

Built with:
- [Tokio](https://tokio.rs/) - Async runtime for Rust
- [Futures](https://github.com/rust-lang/futures-rs) - Async abstractions
- Credits to: sponjibob & cupmister - helped a lot

## 📬 Contact

Questions? Issues? Feel free to open an issue on GitHub!

---

**⭐ If you find this useful, please star the repository!**
