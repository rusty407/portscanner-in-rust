use std::net::{SocketAddr, IpAddr};
use std::time::Duration;
use tokio::net::TcpStream;
use std::env;
use std::io;
use tokio::time::timeout;
use futures::stream::{FuturesUnordered, StreamExt};

pub struct ScanResult {
    pub open_ports: Vec<u16>,
    pub errors: Vec<(u16, io::Error)>,
}

enum PortState {
    Open,
    Closed,
    Error(io::Error),
}

pub struct PortScanner {
    target: IpAddr,
    concurrent_limit: usize,
    timeout: Duration,
}

impl PortScanner {
    pub fn new(target: IpAddr) -> Self {
        Self {
            target,
            concurrent_limit: 10000,
            timeout: Duration::from_millis(450),
        }
    }
    pub fn with_concurrent_limit(mut self, limit: usize) -> Self {
        self.concurrent_limit = limit;
        self
    }

    pub fn with_timeout(mut self, timeout_duration: Duration) -> Self {
        self.timeout = timeout_duration;
        self
    }

    pub async fn run(&self, start_port: u16, end_port: u16) -> ScanResult {
        let mut open_ports = Vec::new();
        let mut errors = Vec::new();
        let mut futures = FuturesUnordered::new();
        // The iterator hands out each port exactly once and can't overflow at 65535.
        let mut ports = start_port..=end_port;

        for port in ports.by_ref().take(self.concurrent_limit) {
            futures.push(self.scan_port(port));
        }

        while let Some((port, state)) = futures.next().await {
            match state {
                PortState::Open => open_ports.push(port),
                PortState::Closed => {}
                PortState::Error(err) => errors.push((port, err)),
            }

            if let Some(port) = ports.next() {
                futures.push(self.scan_port(port));
            }
        }

        open_ports.sort();
        errors.sort_by_key(|(port, _)| *port);
        ScanResult { open_ports, errors }
    }

    async fn scan_port(&self, port: u16) -> (u16, PortState) {
        let socket = SocketAddr::new(self.target, port);

        let state = match timeout(self.timeout, TcpStream::connect(socket)).await {
            // On localhost the kernel can pick the target port as our source port, so the
            // socket "connects" to itself. That's not a real listener.
            Ok(Ok(stream)) if stream.local_addr().ok() == Some(socket) => PortState::Closed,
            Ok(Ok(_)) => PortState::Open,
            Ok(Err(err)) if err.kind() == io::ErrorKind::ConnectionRefused => PortState::Closed,
            // Anything else (e.g. "Too many open files") means the port wasn't actually checked.
            Ok(Err(err)) => PortState::Error(err),
            // No reply in time: closed or filtered by a firewall.
            Err(_) => PortState::Closed,
        };
        (port, state)
    }
}

fn print_usage() {
    println!("Usage: cargo run -- <target> <start_port> <end_port> [timeout_ms] [concurrent_limit]");
    println!("\nExamples:");
    println!("  cargo run -- 127.0.0.1 1 1000                    # Scan ports 1-1000");
    println!("  cargo run -- 192.168.1.1 1 65535                 # Scan ALL ports");
    println!("  cargo run -- 127.0.0.1 1 65535 50 15000          # INSANE SPEED MODE");
    println!("\nSpecial shortcuts:");
    println!("  cargo run -- <ip> all                            # Scans all 65535 ports");
    println!("  cargo run -- <ip> common                         # Scans ports 1-1000");
    println!("\nSpeed presets:");
    println!("  Add 'fast' to use aggressive settings:          # 50ms timeout, 15000 concurrent");
    println!("  cargo run -- <ip> all fast");
}

fn parse_args() -> Result<(IpAddr, u16, u16, Duration, usize), String> {
let args: Vec<String> = env::args().collect();

if args.len() < 3 {
    return Err("You need more arguments".to_string());
}

let target = args[1].parse::<IpAddr>()
    .map_err(|_| format!("Invalid Ip address: {}", args[1]))?;

let (start_port, end_port) = if args.len() == 3 || (args.len() > 3 && args[3] == "fast") {
    match args[2].to_lowercase().as_str() {
        "all" => (1u16, 65535u16),
        "common" => (1u16, 1000u16),
        _ => return Err("Invalid argument. Use 'all', 'common', or specify start and end ports".to_string()),
    }
} else {
    let start = args[2].parse::<u16>()
        .map_err(|_| format!("Invalid start port: {}", args[2]))?;
    let end = args[3].parse::<u16>()
        .map_err(|_| format!("Invalid end port: {}", args[3]))?;

    if start > end {
        return Err("Start port must be <= end port".to_string());
    }
    if start == 0 {
        return Err("Port 0 is not valid".to_string());
    }

    (start, end)
};

let is_fast_mode = args.iter().any(|arg| arg == "fast");

let timeout = if is_fast_mode {
    Duration::from_millis(50)
} else if args.len() > 4 && args[4] != "fast" {
    Duration::from_millis(args[4].parse::<u64>()
        .map_err(|_| format!("Invalid timeout: {}", args[4]))?)
} else {
    Duration::from_millis(500)
};

let concurrent = if is_fast_mode {
    15000
} else if args.len() > 5 {
    args[5].parse::<usize>()
        .map_err(|_| format!("Invalid concurrent limit: {}", args[5]))?
} else {
    1000
};

if concurrent == 0 {
    return Err("Concurrent limit must be at least 1".to_string());
}

Ok((target, start_port, end_port, timeout, concurrent))
}

#[tokio::main]
async fn main() {
    println!("---- Port Scanner ----\n");

    let (target, start_port, end_port, timeout, concurrent) = match parse_args() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("Error: {}\n", err);
            print_usage();
            std::process::exit(1);
        }
    };

    let total_ports = end_port - start_port + 1;

    println!("Target:          {}", target);
    println!("Port range:      {}-{}", start_port, end_port);
    println!("Total ports:     {}", total_ports);
    println!("Concurrent:      {} tasks", concurrent);
    println!("Timeout:         {:?} per port", timeout);

    if concurrent > 10000 {
        println!("\nWARNING: Running in EXTREME MODE!");
        println!("   This may trigger rate limits or firewalls.");
    }

    println!("\nStarting scan...\n");

    let start_time = std::time::Instant::now();

    let scanner = PortScanner::new(target)
        .with_timeout(timeout)
        .with_concurrent_limit(concurrent);

    let ScanResult { open_ports, errors } = scanner.run(start_port, end_port).await;

    let elapsed = start_time.elapsed();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if open_ports.is_empty() {
        println!("No open ports found.");
    } else {
        println!("Found {} open port(s):\n", open_ports.len());

        for port in &open_ports {
            let service = match port {
                21 => "FTP",
                22 => "SSH",
                23 => "Telnet",
                25 => "SMTP",
                53 => "DNS",
                80 => "HTTP",
                110 => "POP3",
                143 => "IMAP",
                443 => "HTTPS",
                445 => "SMB",
                3306 => "MySQL",
                3389 => "RDP",
                5432 => "PostgreSQL",
                5900 => "VNC",
                8080 => "HTTP-Alt",
                _ => "Unknown",
            };
            println!("  Port {:5} - {}", port, service);
        }
    }

    if let Some((port, err)) = errors.first() {
        println!("\nWARNING: {} port(s) could not be scanned, results may be incomplete.", errors.len());
        println!("   First error (port {}): {}", port, err);
        println!("   If it says \"Too many open files\", lower the concurrent limit or run `ulimit -n 65535` first.");
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Scan completed in {:.2?}", elapsed);
    println!("Scanned {} ports at {:.0} ports/sec",
             total_ports,
             total_ports as f64 / elapsed.as_secs_f64());
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn scans_first_port_after_initial_batch() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        // With a limit of 2, `port` is the first one queued after the initial batch.
        let scanner = PortScanner::new("127.0.0.1".parse().unwrap()).with_concurrent_limit(2);
        let result = scanner.run(port - 2, port).await;

        assert!(result.open_ports.contains(&port));
    }

    #[tokio::test]
    async fn range_ending_at_65535_does_not_overflow() {
        let scanner = PortScanner::new("127.0.0.1".parse().unwrap()).with_concurrent_limit(10);
        scanner.run(65530, 65535).await;
    }
}
