use std::net::{SocketAddr, IpAddr};
use std::time::Duration;
use tokio::net::TcpStream;
use std::io;
use tokio::time::{sleep_until, timeout, Instant};
use futures::stream::{FuturesUnordered, StreamExt};
use clap::{CommandFactory, Parser, error::ErrorKind};

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
    rate_interval: Option<Duration>,
}

impl PortScanner {
    pub fn new(target: IpAddr) -> Self {
        Self {
            target,
            concurrent_limit: 1000,
            timeout: Duration::from_millis(500),
            rate_interval: None,
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

    /// Start at most `per_second` connections per second. Must be at least 1.
    pub fn with_rate(mut self, per_second: u32) -> Self {
        self.rate_interval = Some(Duration::from_secs(1) / per_second);
        self
    }

    pub async fn run(&self, ports: Vec<u16>) -> ScanResult {
        let mut open_ports = Vec::new();
        let mut errors = Vec::new();
        let mut futures = FuturesUnordered::new();
        let mut ports = ports.into_iter();

        // With a rate limit, each connection gets a start time at least `interval` after the
        // previous one. If the scan falls behind, it starts from "now" instead of bursting to
        // catch up.
        let mut next_start = Instant::now();
        let mut schedule = || {
            self.rate_interval.map(|interval| {
                let start_at = next_start.max(Instant::now());
                next_start = start_at + interval;
                start_at
            })
        };

        for port in ports.by_ref().take(self.concurrent_limit) {
            futures.push(self.scan_port(port, schedule()));
        }

        while let Some((port, state)) = futures.next().await {
            match state {
                PortState::Open => open_ports.push(port),
                PortState::Closed => {}
                PortState::Error(err) => errors.push((port, err)),
            }

            if let Some(port) = ports.next() {
                futures.push(self.scan_port(port, schedule()));
            }
        }

        open_ports.sort();
        errors.sort_by_key(|(port, _)| *port);
        ScanResult { open_ports, errors }
    }

    async fn scan_port(&self, port: u16, start_at: Option<Instant>) -> (u16, PortState) {
        if let Some(start_at) = start_at {
            sleep_until(start_at).await;
        }

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

/// Fast async TCP port scanner
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// IP address to scan (IPv4 or IPv6)
    target: IpAddr,

    /// Ports to scan: "all", "common" (1-1000), or a list like 22,80,443,8000-9000
    #[arg(short, long, default_value = "common")]
    ports: String,

    /// How long to wait for each port to answer, in milliseconds
    #[arg(short, long, value_name = "MS", default_value_t = 500, value_parser = clap::value_parser!(u64).range(1..))]
    timeout: u64,

    /// How many connection attempts can run at the same time
    #[arg(short, long, value_name = "N", default_value_t = 1000, value_parser = clap::value_parser!(u32).range(1..))]
    concurrency: u32,

    /// Start at most this many connections per second [default: no limit]
    #[arg(short, long, value_name = "N", value_parser = clap::value_parser!(u32).range(1..))]
    rate: Option<u32>,

    /// Scan ports in random order instead of 1, 2, 3, ...
    #[arg(short, long)]
    shuffle: bool,

    /// Speed preset: 50ms timeout and 15000 connections at once
    #[arg(long, conflicts_with_all = ["timeout", "concurrency"])]
    fast: bool,
}

/// Parses "all", "common", or a comma-separated list of ports and ranges like "22,80,8000-9000".
/// Returns the ports sorted, without duplicates.
fn parse_ports(spec: &str) -> Result<Vec<u16>, String> {
    let mut ports = Vec::new();

    for part in spec.split(',').map(str::trim) {
        match part {
            "all" => ports.extend(1..=65535),
            "common" => ports.extend(1..=1000),
            _ => {
                let (start, end) = match part.split_once('-') {
                    Some((start, end)) => (parse_port(start)?, parse_port(end)?),
                    None => (parse_port(part)?, parse_port(part)?),
                };
                if start > end {
                    return Err(format!("invalid range {part}: start is after end"));
                }
                ports.extend(start..=end);
            }
        }
    }

    ports.sort_unstable();
    ports.dedup();
    Ok(ports)
}

fn parse_port(s: &str) -> Result<u16, String> {
    match s.trim().parse::<u16>() {
        Ok(0) | Err(_) => Err(format!("invalid port '{s}' (must be 1-65535)")),
        Ok(port) => Ok(port),
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let mut ports = match parse_ports(&cli.ports) {
        Ok(ports) => ports,
        Err(err) => Cli::command().error(ErrorKind::ValueValidation, err).exit(),
    };
    if cli.shuffle {
        fastrand::shuffle(&mut ports);
    }

    let (timeout, concurrent) = if cli.fast {
        (Duration::from_millis(50), 15000)
    } else {
        (Duration::from_millis(cli.timeout), cli.concurrency as usize)
    };

    let total_ports = ports.len();

    println!("---- Port Scanner ----\n");
    println!("Target:          {}", cli.target);
    println!("Ports:           {}", cli.ports);
    println!("Total ports:     {}", total_ports);
    println!("Concurrent:      {} tasks", concurrent);
    println!("Timeout:         {:?} per port", timeout);
    match cli.rate {
        Some(rate) => println!("Rate limit:      {} connections/sec (takes at least {:.0?})",
                               rate,
                               Duration::from_secs_f64(total_ports as f64 / rate as f64)),
        None => println!("Rate limit:      none"),
    }
    println!("Order:           {}", if cli.shuffle { "random" } else { "sequential" });

    if concurrent > 10000 {
        println!("\nWARNING: Running in EXTREME MODE!");
        println!("   This may trigger rate limits or firewalls.");
    }

    println!("\nStarting scan...\n");

    let start_time = std::time::Instant::now();

    let mut scanner = PortScanner::new(cli.target)
        .with_timeout(timeout)
        .with_concurrent_limit(concurrent);
    if let Some(rate) = cli.rate {
        scanner = scanner.with_rate(rate);
    }

    let ScanResult { open_ports, errors } = scanner.run(ports).await;

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
        let result = scanner.run((port - 2..=port).collect()).await;

        assert!(result.open_ports.contains(&port));
    }

    #[tokio::test]
    async fn range_ending_at_65535_does_not_overflow() {
        let scanner = PortScanner::new("127.0.0.1".parse().unwrap()).with_concurrent_limit(10);
        scanner.run(parse_ports("65530-65535").unwrap()).await;
    }

    #[tokio::test]
    async fn rate_limit_spaces_out_connections() {
        let scanner = PortScanner::new("127.0.0.1".parse().unwrap()).with_rate(100);
        let start = Instant::now();
        scanner.run((40000..40020).collect()).await;

        // 20 connections at 100/sec: the last one can't start before 190ms.
        assert!(start.elapsed() >= Duration::from_millis(190));
    }

    #[test]
    fn parses_port_lists() {
        assert_eq!(parse_ports("80,22, 8000-8002,22").unwrap(), vec![22, 80, 8000, 8001, 8002]);
        assert_eq!(parse_ports("common").unwrap(), (1..=1000).collect::<Vec<u16>>());
        assert_eq!(parse_ports("all").unwrap().len(), 65535);
    }

    #[test]
    fn rejects_bad_ports() {
        for spec in ["0", "65536", "90-80", "abc", "", "80,", "1-2-3"] {
            assert!(parse_ports(spec).is_err(), "{spec:?} should be rejected");
        }
    }
}
