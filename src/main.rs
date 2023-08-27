use bpaf::Bpaf;
use std::io::{self, Write};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::mpsc::{channel, Sender};
use tokio::net::TcpStream; // Note this is Asynchronous !
use tokio::task; // "Tasks are green threads in the Tokio system"

// Credit : Tensor Programming - https://www.youtube.com/watch?v=RhFZxkxkeIc&t=705s

// Channel : Creates a new asynchronous channel,
// returning the sender/receiver halves.
// mpsc = multiple producers, single consumer

// Max possible port number
const MAX: u16 = 65535;

// Address fallback.
const IPFALLBACK: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

// get cli arguments
#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
pub struct Arguments {
    #[bpaf(short, long, fallback(IPFALLBACK))]
    pub address: IpAddr,

    // low port
    #[bpaf(
        long("start"),
        short('s'),
        guard(start_port_guard, "Must be greater than 0"),
        fallback(1_u16)
    )]
    pub start_port: u16,

    // high port
    #[bpaf(
        long("end"),
        short('e'),
        guard(end_port_guard, "Must be less than or equal to 65535"),
        fallback(MAX)
    )]
    pub end_port: u16,
}

// Borrow the input!
fn start_port_guard(input: &u16) -> bool {
    *input > 0
}

// Borrow the input!
fn end_port_guard(input: &u16) -> bool {
    *input < MAX
}

// scan ports
async fn scan(tx: Sender<u16>, port: u16, addr: IpAddr) {
    match TcpStream::connect(format!("{}:{}", addr, port)).await {
        Ok(_) => {
            println!(".");
            io::stdout().flush().unwrap();
            tx.send(port).unwrap();
        }
        Err(_) => {}
    }
}

// Main
#[tokio::main]
async fn main() {
    // The macro the we implemented on the Arguments struct
    // creates a FUNCTION of the SAME NAME
    // collects values from the parser and puts them into the struct
    let opts: Arguments = arguments().run();

    // Initialize the channel.
    let (tx, rx) = channel();
    // Iterate through all of the ports (based on user input)
    // so that we can spawn a single task for each.
    // Much faster than before because it uses green threads instead of OS threads.
    // Tasks are green threads in the "Tokio system"
    for i in opts.start_port..opts.end_port {
        let tx = tx.clone();
        task::spawn(async move { scan(tx, i, opts.address).await });
    }
    let mut out = vec![];
    drop(tx);

    for p in rx {
        out.push(p);
    }

    println!("");
    out.sort();
    for v in out {
        // Iterate through the outputs and print them out as being open.
        println!("{} is open", v);
    }
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scan_successful_connection() {
        // Start a TCP server on an available port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:8880")
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();

        // Create a channel for testing
        let (tx, _rx) = channel();

        // Try to connect using the scan function
        let result = task::spawn(scan(
            tx,
            addr.port(),
            // addr.ip(),
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        ))
        .await;

        // Ensure that the connection attempt was successful
        assert!(result.is_ok());
    }
}

/*

In most cases, the size impact of unit tests is relatively small compared 
to the rest of your application code and its dependencies. 
Rust's strong focus on zero-cost abstractions 
and efficient code generation helps mitigate any potential size increase.

LTO - link time optimization

https://nnethercote.github.io/perf-book/build-configuration.html
https://doc.rust-lang.org/rustc/codegen-options/index.html#lto
https://doc.rust-lang.org/rustc/codegen-options/index.html#prefer-dynamic

Spawning a Task: When you call task::spawn,
you're creating a new asynchronous task that
will run concurrently
with the rest of your program.
This means that the spawned task will not block the execution of
the main thread or other tasks.

cargo build --profile release-lto

# [profile.release-lto]
# inherits = "release"
# lto = true

*/
