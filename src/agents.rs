use std::net::SocketAddr;
use std::io;


use remote_file_system::client::Client;

pub struct Agent {
    client: Client,
    id: u32,
}

impl Agent {
    pub fn new(id: u32, server_addr: &SocketAddr, addr: SocketAddr) -> Self {
        let client = Client::new(*server_addr, addr, id);
        Self {client, id}
    }

    pub fn run(&mut self) {
        println!("Agent {} started on address {}",
            self.id,
            self.client.get_client_address()
        );

        loop {
            println!("Type 'write', 'read', 'open', 'close', or 'exit' to quit.");

            // 1. Create a mutable, empty String to hold the input
            let mut input_buffer = String::new();

            // 2. Get the standard input handle and call read_line()
            let read_result = io::stdin().read_line(&mut input_buffer);

            // 3. Handle the result (read_line returns a Result)
            match read_result {
                Ok(_) => {
                    // Trim whitespace (including the newline character) before printing
                    let trimmed_input = input_buffer.trim(); 
                    match trimmed_input {
                        "write" => {
                            println!("Agent {} preparing to write. Enter text to write:", self.id);
                            let mut text_to_write = String::new();
                            io::stdin().read_line(&mut text_to_write).expect("Failed to read text to write");

                            println!("File ID:");
                            let mut file_id = String::new();
                            io::stdin().read_line(&mut file_id).expect("Failed to read file ID");

                            println!("Agent {} writing: {}", self.id, text_to_write.trim());
                            let fd: i32 = file_id.trim().parse().unwrap_or(0);
                            match self.client.escreve(fd, 0, &mut text_to_write.as_bytes().to_vec(), text_to_write.len()) {
                                0 => println!("Agent {} wrote to file {}", self.id, file_id),
                                i => println!("Error: Agent {} failed to write to file {}: {}", self.id, file_id, i),
                            }
                            continue; // Exit the run method to stop the agent
                        },
                        "read" => {
                            println!("File ID:");
                            let mut file_id = String::new();
                            io::stdin().read_line(&mut file_id).expect("Failed to read file ID");

                            println!("Agent {} reading from file {}", self.id, file_id);
                            let fd: i32 = file_id.trim().parse().unwrap_or(0);
                            let mut buffer = vec![];
                            match self.client.le(fd, 0, &mut buffer, 128) {
                                -1 => println!("Error: Agent {} failed to read from file {}", self.id, file_id),
                                size => {
                                    let msg = String::from_utf8_lossy(&buffer);
                                    println!("Agent {} read data: {:?} with size {}", self.id, msg, size);
                                },
                            }
                            continue; // Exit the run method to stop the agent
                        },
                        "open" => {
                            println!("File name to open:");
                            let mut file_id = String::new();
                            io::stdin().read_line(&mut file_id).expect("Failed to read filename");
                            let fd = file_id.trim().parse().unwrap_or(0);
                            println!("Agent {} opening file {}", self.id, file_id.trim());
                            match self.client.abre(fd, file_id.trim().to_string()) {
                                0 => println!("Agent {} opened file {}", self.id, file_id.trim()),
                                i => println!("Error: Agent {} failed to open file {}: {}", self.id, file_id.trim(), i),
                            }
                            continue; // Exit the run method to stop the agent
                                
                        }
                        "close" => {
                            println!("File ID to close:");
                            let mut file_id = String::new();
                            io::stdin().read_line(&mut file_id).expect("Failed to read file ID");

                            let fd = file_id.trim().parse().unwrap_or(0);
                            println!("Agent {} closing file {}", self.id, file_id.trim());
                            match self.client.fecha(fd) {
                                0 => println!("Agent {} closed file {}", self.id, file_id.trim()),
                                i => println!("Error: Agent {} failed to close file {}: {}", self.id, file_id.trim(), i),
                            }
                            continue; // Exit the run method to stop the agent
                        },
                        "exit" => {
                            println!("Agent {} exiting.", self.id);
                            break; // Exit the run method to stop the agent
                        },
                        _ => {
                            println!("Agent {} received input: {}", self.id, trimmed_input);
                            // Here you can add code to process the input as needed
                            continue;
                        }
                    }
                }
                Err(error) => {
                    eprintln!("Error reading input: {}", error);
                }
            }
        }

        





        // Example operations
        // let filename = format!("file.txt");// format!("file_{}.txt", self.id);
        // let fd = 0; // self.id as i32; // Using id as file descriptor for simplicity

        // // Open file
        // println!("Agent {} opening file {}", self.id, filename);
        // match self.client.abre(fd, filename.clone()) {
        //     0 => println!("Agent {} opened file {}", self.id, filename),
        //     i => println!("Error: Agent {} failed to open file {}: {}", self.id, filename, i),
        // }

        // // Write to file
        // let data = format!("Hello from agent {}", self.id);
        // println!("Agent {} writing to file {}", self.id, filename);
        // match self.client.escreve(fd, 0, &mut data.as_bytes().to_vec(), data.len()) {
        //     0 => println!("Agent {} wrote to file {}", self.id, filename),
        //     i => println!("Error: Agent {} failed to write to file {}: {}", self.id, filename, i),
        // }
        
        // // Read from file
        // let mut buffer = vec![];
        // println!("Agent {} reading from file {}", self.id, filename);
        // match self.client.le(fd, 0, &mut buffer, data.len()) {
        //     -1 => println!("Error: Agent {} failed to read from file {}", self.id, filename),
        //     size => {
        //         let msg = String::from_utf8_lossy(&buffer[..size as usize]);
        //         println!("Agent {} read data: {:?}", self.id, msg);
        //     },
        // }

        // // Close file
        // println!("Agent {} closing file {}", self.id, filename);
        // match self.client.fecha(fd) {
        //     0 => println!("Agent {} closed file {}", self.id, filename),
        //     i => println!("Error: Agent {} failed to close file {}: {}", self.id, filename, i),
        // }
    }
    
}
