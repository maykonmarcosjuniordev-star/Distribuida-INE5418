// https://doc.rust-lang.org/rust-by-example/scope/lifetime/explicit.html
use std::thread;


use remote_file_system::server::Server;
mod agents;
use agents::Agent;



fn main() {
    let server = Server::new("127.0.0.1", 8080);
    let server_addr = server.get_address();
    let server_handle = {
        thread::spawn(move || {
            server.run();
        })
    };
    let mut agents_handles = vec![];
    for i in 0..2 {
        let addr = format!("127.0.0.1:{}", 8081 + i).parse().expect("Failed to parse agent address");
        println!("Creating agent {} on address {}", i, addr);
        let agent = Agent::new(i, &server_addr, addr);
        let handle = thread::spawn(move || {
            agent.run();
        });
        agents_handles.push(handle);
    }
    for handle in agents_handles {
        handle.join().expect("Failed to join agent thread");
    }
    server_handle.join().expect("Failed to join server thread");
}
