use std::net::{SocketAddr, IpAddr};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use futures::stream::{FuturesUnordered, StreamExt};

struct PortScanner {
    target: IpAddr,
    start_port: u16,
    end_port: u16,
    timeout: Duration,
    concurrent_limit: usize,
}

impl PortScanner {
    fn new( target: IpAddr, start_port: u16, end_port: u16 ) -> Self {
        Self {
            target,
            start_port,
            end_port,
            timeout: Duration::from_millis(500),
            concurrent_limit: 100,
        }
    }

    async fn scan(&self) -> Vec<u16> {
        let mut open_ports = Vec::new();
        let mut futures = FuturesUnordered::new();
        let mut port = self.start_port;


        for _ in 0..self.concurrent_limit.min((self.end_port - self.start_port - self.start_port + 1) as usize) {
            if port <= self.end_port {
                futures.push(self.check_port(port));
                port += 1;
            }
    }

        while let Some(result) = futures.next().await {
            if let Some(open_port) = result {
                open_ports.push(open_port);
            }

            if port <= self.end_port {
                futures.push(self.check_port(port));
                port += 1;
            }
        }

        open_ports.sort();
        open_ports
}
    async fn check_port(&self, port: u16) -> Option<u16> {
        let socket_addr = SocketAddr::new(self.target, port);

        match timeout(self.timeout, TcpStream::connect(socket_addr)).await {
            Ok(Ok(_)) => Some(port),
            _ => None,
        }
    }
}

#[tokio::main]
async fn main() {
    println!("==== Rust Port Scanner ====\n");

    let target = "192.168.1.1".parse::<IpAddr>().unwrap();

    println!("Scanning target: {}", target);
    println!("Port range: 1-7000");
    println!("Starting scan... \n");

    // let start_time = std::time::Instant::now();

    let scanner = PortScanner::new(target, 1, 7000);
    let open_ports = scanner.scan().await;

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
