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
        }
    }
}

fn main() {
    println("Hello World");
}
