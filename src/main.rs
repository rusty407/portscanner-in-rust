use std::net::{SocketAddr, IpAddr};
use std::time::Duration;
use tokio::net::TcpStream;
use std::env;
use tokio::time::timeout;
use futures::stream::{FuturesUnordered, StreamExt};

pub const LOWEST_PORT_NUMBER: u16 = 1;
pub const HIGHEST_PORT_NUMBER: u16 = 65535;

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
            timeout: Duration::from_millis(50),
        }
    }
    pub fn with_concurrent_limit(mut self, limit: usize) -> Self {
        self.concurrent_limit = limit;
        self
    }

    pub fn with_timeout(mut self, timeout_duration: usize) -> Self {
        self.timeout = timeout_duration;
        self
    }

    pub async fn run(&self, start_port: u16, end_port: u16) -> Vec<u16> {
        let mut open_ports = Vec::new();
        let mut futures = FuturesUnordered::new();
        let mut current_port = start_port;

        for _ in 0..self.concurrent_limit.min((end_port - start_port + 1) as usize) {
            if current_port <= end_port {
                futures.push(self.scan_port(current_port));
                current_port += 1;
            }
        }

        while let Some(result) = futures.next().await {
            if let Some(port) = result {
                open_ports.push(port);
            }

            if current_port <= end_port {
                futures.push(self.scan_port(current_port));
                current_port += 1;
            }
        }

        open_ports.sort();
        open_ports
    }

    async fn scan_port(&self, port: u16) -> Option<u16> {
        let socket = SocketAddr::new(self.target, port);

        match timeout(self.timeout, TcpStream::connect(socket)).await {
            Ok(Ok(_)) => Some(port),
            _ => None,
        }
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

#[tokio::main]
async fn main() {
    println!("==== Rust Port Scanner ====\n");

    let target = "98.84.224.111".parse::<IpAddr>().unwrap();

    println!("Scanning target: {}", target);
    println!("Port range: 1-65535");
    println!("Starting scan... \n");

    // let start_time = std::time::Instant::now();

    let scanner = PortScanner::new(target);
    let open_ports = scanner.run(1, 65535).await;

    // let elapsed = start_time.elapsed();

    if open_ports.is_empty() {
        println!("No open ports found.");
    } else {
        println!("Found {} open port(s):", open_ports.len());
        for port in &open_ports {
            println!("  Port {} is OPEN", port);
        }
    }
}
