// https://doc.rust-lang.org/rust-by-example/scope/lifetime/explicit.html
use std::thread;


use remote_file_system::server::Server;
mod agents;
use agents::Agent;

fn is_number(s: &str) -> bool {
    s.parse::<i32>().is_ok()
}

fn main() {

    let server_a = Server::new("127.0.0.1", 8080);
    let server_addr = server_a.get_address();
    
    let args: Vec<String> = std::env::args().collect();
    let mut agents_handles = vec![];
    let mut server_handles = vec![];


    for arg in &args {
        println!("Arg: {}", arg);
        if arg == "server" {
            let mut server = Server::new("127.0.0.1", 8080);
            let server_handle = {
                thread::spawn(move || {
                    server.run();
                })
            };
            server_handles.push(server_handle);
            continue;
        }

        if is_number(arg) {
            let id: u32 = arg.parse().unwrap();
            let addr = format!("127.0.0.1:{}", 8081 + id).parse().expect("Failed to parse agent address");
            println!("Creating agent {} on address {}", id, addr);
            let mut agent = Agent::new(id, &server_addr, addr);
            let handle = thread::spawn(move || {
                agent.run();
            });
            agents_handles.push(handle);
        }
    }

    // for i in 0..5 {
    //     let addr = format!("127.0.0.1:{}", 8081 + i).parse().expect("Failed to parse agent address");
    //     println!("Creating agent {} on address {}", i, addr);
    //     let mut agent = Agent::new(i, &server_addr, addr);
    //     let handle = thread::spawn(move || {
    //         agent.run();
    //     });
    //     agents_handles.push(handle);
    // }
    for handle in agents_handles {
        handle.join().expect("Failed to join agent thread");
    }
    for handle in server_handles {
        handle.join().expect("Failed to join server thread");
    }
}
