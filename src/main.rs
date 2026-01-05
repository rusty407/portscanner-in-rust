use std::net::{TcpStream, SocketAddr, IpAddr};
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::thread;

struct PortScanner {
    target: IpAddr,
    start_port: u16,
    end_port: u16,
    timeout: Duration,
    threads: usize,
}
impl PortScanner {
    fn new( target: IpAddr, start_port: u16, end_port: u16 ) -> Self {
        Self {
            target,
            start_port,
            end_port,
            timeout: Duration::from_millis(500),
            threads: 100,
        }
    }
    fn scan(&self) -> Vec<u16> {
        let open_ports = Arc::new(Mutex::new(Vec::new()));
        let mut handles = vec![];

        let total_ports = (self.end_port - self.start_port + 1) as usize;
        let chunk_size = (total_ports + self.threads - 1) / self.threads;

        for thread_id in 0..self.threads {
            let start = self.start_port + (thread_id * chunk_size) as u16;
            let end = std::cmp::min(
                start + chunk_size as u16 - 1,
                self.end_port
            );

            if start > self.end_port {
                break;
            }

            let target = self.target;
            let timeout = self.timeout;
            let open_ports = Arc::clone(&open_ports);

            let handle = thread::spawn(move || {
                for port in start..=end {
                    if Self::check_port(target, port, timeout) {
                        let mut ports = open_ports.lock().unwrap();
                        ports.push(port);
                    }
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let mut result = open_ports.lock().unwrap().clone();
        result.sort();
        result
    }

    fn check_port(target: IpAddr, port: u16, timeout: Duration) -> bool {
        let socket_addr = SocketAddr::new(target, port);
        TcpStream::connect_timeout(&socket_addr, timeout).is_ok()
    }
}

fn main() {
    println!("==== Rust Port Scanner ====");

    let target = "192.168.1.1".parse::<IpAddr>().unwrap();

    println!("Scanning target: {}", target);
    println!("Port range: 1-1000");
    println!("Starting scan... \n");

    let scanner = PortScanner::new(target, 1, 1000);
    let open_ports = scanner.scan();

    if open_ports.is_empty() {
        println!("No open ports found.");
    } else {
        println!("Found {} open port(s):", open_ports.len());
        for port in open_ports {
            println!("  Port {} is OPEN", port);
        }
    }
}
