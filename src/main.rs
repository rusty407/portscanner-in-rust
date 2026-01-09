use std::net::{SocketAddr, IpAddr};
use std::time::Duration;
use tokio::net::TcpStream;
use std::env;
use tokio::time::timeout;
use futures::stream::{FuturesUnordered, StreamExt};

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

            if current_port < end_port {
                current_port += 1;
                futures.push(self.scan_port(current_port));
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
    Duration::from_millis(450)
};

let concurrent = if is_fast_mode {
    15000
} else if args.len() > 5 {
    args[5].parse::<usize>()
        .map_err(|_| format!("Invalid concurrent limit: {}", args[5]))?
} else {
    10000
};
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

    let open_ports = scanner.run(start_port, end_port).await;

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

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Scan completed in {:.2?}", elapsed);
    println!("Scanned {} ports at {:.0} ports/sec",
             total_ports,
             total_ports as f64 / elapsed.as_secs_f64());
}
