// https://doc.rust-lang.org/rust-by-example/scope/lifetime/explicit.html
use std::{thread::{self, sleep}, time::Duration};


use remote_file_system::server::Server;
mod agents;
use agents::Agent;


fn create_agents(num_agents: u32, server_addr: &std::net::SocketAddr) -> Vec<Agent> {
    let mut agents = vec![];
    for i in 0..num_agents {
        let addr = format!("127.0.0.1:{}", 8081 + i).parse().expect("Failed to parse agent address");
        println!("Creating agent {} on address {}", i, addr);
        let agent = Agent::new(i, &server_addr, addr);
        agents.push(agent);
    }
    return agents;    
}

fn test_1(server_addr: &std::net::SocketAddr) {
    println!("Starting Test 1: Simple write and read test.");
    let agents = create_agents(2, server_addr);

    let agent0 = &agents[0];
    let agent1 = &agents[1];

    agent0.abre(0, "file.txt".to_string());
    agent1.abre(0, "file.txt".to_string());

    let mut agent0_write_buffer = "Hello from agent 0\n".as_bytes().to_vec();
    let tamanho = agent0_write_buffer.len();
    agent0.escreve(0, 0, &mut agent0_write_buffer, tamanho);

    let mut agent1_read_buffer = vec![];
    agent1.le(0, 0, &mut agent1_read_buffer, 128);

    assert_eq!(&agent0_write_buffer[..], &agent1_read_buffer[..tamanho]);

    agent0.fecha(0);
    agent1.fecha(0);
    println!("Test 1 completed successfully.\n");
}

fn test_2(server_addr: &std::net::SocketAddr){
    println!("Starting Test 2: Simple cache invalidation test.");
    let agents = create_agents(2, server_addr);

    let agent0 = &agents[0];
    let agent1 = &agents[1];

    agent0.abre(0, "file.txt".to_string());
    agent1.abre(0, "file.txt".to_string());

    let mut agent0_read_buffer = vec![];
    agent0.le(0, 0, &mut agent0_read_buffer, 128);

    let mut agent1_write_buffer = "Hello for test 2 from agent 1\n".as_bytes().to_vec();
    let tamanho = agent1_write_buffer.len();
    agent1.escreve(0, 0, &mut agent1_write_buffer, tamanho);

    let mut agent0_read_buffer_after = vec![];
    agent0.le(0, 0, &mut agent0_read_buffer_after, tamanho);

    assert_ne!(&agent0_read_buffer[..], &agent0_read_buffer_after[..tamanho]);
    assert_eq!(&agent1_write_buffer[..], &agent0_read_buffer_after[..tamanho]);

    agent0.fecha(0);
    agent1.fecha(0);

    println!("Test 2 completed successfully.\n");
}

fn test_3(server_addr: &std::net::SocketAddr){
    println!("Starting Test 3: Concurrent writes and reads test.");
    let agents = create_agents(5, server_addr);

    let mut count = 0;
    let handles: Vec<_> = agents.into_iter().map(|agent| {
        thread::spawn(move || {
            let filename = "file.txt".to_string();
            let fd = 0;

            // Open file
            println!("Agent {} opening file {}", agent.get_id(), filename);
            match agent.abre(fd, filename.clone()) {
                0 => {
                    println!("Agent {} opened file {}", agent.get_id(), filename);
                },
                i => {
                    println!("Error: Agent {} failed to open file {}: {} --> Stopping", agent.get_id(), filename, i);
                },
            }

            // Write to file
            let data = format!("Hello from agent {}\n", agent.get_id());
            println!("Agent {} writing {} to file {}", agent.get_id(), data, filename);
            let buffer = data.as_bytes().to_vec();
            let tamanho = buffer.len();
            let mut posicao = 0;
            for _ in 0..2 {
                match agent.escreve(fd, posicao, &mut buffer.clone(), tamanho) {
                    0 => {
                        println!("Agent {} wrote {} to file {}", agent.get_id(), data, filename);
                        posicao += tamanho as u64;
                    },
                    i => {
                        println!("Error: Agent {} failed to write to file {}: {}, reopening", agent.get_id(), filename, i);
                        agent.abre(fd, filename.clone());
                    },
                }
            }

            // Read from file
            let mut read_buffer = vec![];
            match agent.le(fd, 0, &mut read_buffer, 128) {
                0 => {
                    println!("Agent {} read from file {}: {}", agent.get_id(), filename, String::from_utf8_lossy(&read_buffer));
                },
                i => {
                    println!("Error: Agent {} failed to read from file {}: {}", agent.get_id(), filename, i);
                },
            }

            // Close file
            match agent.fecha(fd) {
                0 => {
                    println!("Agent {} closed file {}", agent.get_id(), filename);
                },
                i => {
                    println!("Error: Agent {} failed to close file {}: {}", agent.get_id(), filename, i);
                },
            }
        })
    }).collect();
    for handle in handles {
        handle.join().expect("Failed to join agent thread");
        count += 1;
    }
    println!("Test 3 completed successfully with {} agents.\n", count);
}

fn test_4(server_addr: &std::net::SocketAddr) {
    println!("Starting Test 4: Large write and read test with cache coherence.");
    let agents = create_agents(2, server_addr);

    let agent0 = &agents[0];
    let agent1 = &agents[1];

    agent0.abre(0, "file.txt".to_string());
    agent1.abre(0, "file.txt".to_string());

    let mut agent0_write_buffer = "Lorem ipsum dolor sit amet, consectetuer adipiscing elit. Aenean commodo ligula eget dolor. Aenean massa. Cum sociis natoque penatibus et magnis dis parturient montes, nascetur ridiculus mus. Donec quam felis, ultricies nec, pellentesque eu, pretium quis, sem. Nulla consequat massa quis enim. Donec pede justo, fringilla vel, aliquet nec, vulputate eget, arcu. In enim justo, rhoncus ut, imperdiet a, venenatis vitae, justo. Nullam dictum felis eu pede mollis pretium. Integer tincidunt. Cras dapibus. Vivamus e"
        .as_bytes().to_vec();
    let tamanho = agent0_write_buffer.len();
    agent0.escreve(0, 0, &mut agent0_write_buffer, tamanho);

    let mut agent1_read_buffer = vec![];
    agent1.le(0, 0, &mut agent1_read_buffer, 64);

    assert_eq!(&agent0_write_buffer[..64], &agent1_read_buffer[..64]);

    let mut agent0_write_buffer_1 = "0000000000000000000000000000000000000000000000000000000000000000"
        .as_bytes().to_vec();
    let tamanho_1 = agent0_write_buffer_1.len();
    agent0.escreve(0, 64, &mut agent0_write_buffer_1, tamanho_1);

    agent1.le(0, 0, &mut agent1_read_buffer, 64);
    assert_eq!(&agent0_write_buffer[..64], &agent1_read_buffer[64..128]);

    agent1.le(0, 64, &mut agent1_read_buffer, 64);
    assert_eq!(&agent0_write_buffer_1[..], &agent1_read_buffer[128..]);

    agent0.fecha(0);
    agent1.fecha(0);

    println!("Test 4 completed successfully.\n");
}

fn test_5(server_addr: &std::net::SocketAddr) {
    println!("Starting Test 4: Large write and read test with cache coherence and overlap.");
    let agents = create_agents(2, server_addr);

    let agent0 = &agents[0];
    let agent1 = &agents[1];

    agent0.abre(0, "file.txt".to_string());
    agent1.abre(0, "file.txt".to_string());

    let mut agent0_write_buffer = "Lorem ipsum dolor sit amet, consectetuer adipiscing elit. Aenean commodo ligula eget dolor. Aenean massa. Cum sociis natoque penatibus et magnis dis parturient montes, nascetur ridiculus mus. Donec quam felis, ultricies nec, pellentesque eu, pretium quis, sem. Nulla consequat massa quis enim. Donec pede justo, fringilla vel, aliquet nec, vulputate eget, arcu. In enim justo, rhoncus ut, imperdiet a, venenatis vitae, justo. Nullam dictum felis eu pede mollis pretium. Integer tincidunt. Cras dapibus. Vivamus e"
        .as_bytes().to_vec();
    let tamanho = agent0_write_buffer.len();
    agent0.escreve(0, 0, &mut agent0_write_buffer, tamanho);

    let mut agent1_read_buffer = vec![];
    agent1.le(0, 0, &mut agent1_read_buffer, 64);

    assert_eq!(&agent0_write_buffer[..64], &agent1_read_buffer[..64]);

    let mut agent0_write_buffer_1 = "0000000000000000000000000000000000000000000000000000000000000000"
        .as_bytes().to_vec();
    let tamanho_1 = agent0_write_buffer_1.len();
    agent0.escreve(0, 32, &mut agent0_write_buffer_1, tamanho_1);

    let mut agent1_read_buffer_1 = vec![];
    agent1.le(0, 0, &mut agent1_read_buffer_1, 64);
    assert_ne!(&agent1_read_buffer[..64], &agent1_read_buffer_1[..64]);

    let mut agent1_read_buffer_2 = vec![];
    agent1.le(0, 32, &mut agent1_read_buffer_2, 64);
    assert_eq!(&agent0_write_buffer_1, &agent1_read_buffer_2);

    agent0.fecha(0);
    agent1.fecha(0);

    println!("Test 5 completed successfully.\n");
}

fn run_tests() {
    let server = Server::new("127.0.0.1", 8080);
    let server_addr = server.get_address();
    let server_handle = {
        thread::spawn(move || {
            server.run();
        })
    };

    sleep(Duration::from_millis(500)); // Give server time to start
    test_1(&server_addr);
    test_2(&server_addr);
    test_3(&server_addr);
    test_4(&server_addr);
    test_5(&server_addr);


    server_handle.join().expect("Failed to join server thread");

}

fn is_number(s: &str) -> bool {
    s.parse::<i32>().is_ok()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    println!("Args len: {}", args.len());
    if args.len() == 1 {
        run_tests();
    } else {
        run_manually();
    }
    // simple_test();
}

fn run_manually() {
    let server_a = Server::new("127.0.0.1", 8080);
    let server_addr = server_a.get_address();
    
    let args: Vec<String> = std::env::args().collect();
    let mut agents_handles = vec![];
    let mut server_handles = vec![];
    for arg in &args {
        println!("Arg: {}", arg);
        if arg == "server" {
            let server = Server::new("127.0.0.1", 8080);
            let server_handle = {
                thread::spawn(move || {
                    server.run();
                })
            };
            server_handles.push(server_handle);
            break;
        }

        if is_number(arg) {
            let id: u32 = arg.parse().unwrap();
            let addr = format!("127.0.0.1:{}", 8081 + id).parse().expect("Failed to parse agent address");
            println!("Creating agent {} on address {}", id, addr);
            let agent = Agent::new(id, &server_addr, addr);
            let handle = thread::spawn(move || {
                agent.run();
            });
            agents_handles.push(handle);
            break;
        }
    }

    for handle in agents_handles {
        handle.join().expect("Failed to join agent thread");
    }
    for handle in server_handles {
        handle.join().expect("Failed to join server thread");
    }
}

fn simple_test() {
    let server = Server::new("127.0.0.1", 8080);
    let server_addr = server.get_address();
    let server_handle = {
        thread::spawn(move || {
            server.run();
        })
    };
    // checks the command line arguments for number of agents
    let mut num_agents = 0; // default
    if let Some(arg) = std::env::args().nth(1) {
        if let Ok(n) = arg.parse::<u32>() {
            if n > 0 {
                println!("Creating {} agents", n);
                num_agents = n;
            }
        }
    }
    let mut agents_handles = vec![];
    for i in 0..num_agents {
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
