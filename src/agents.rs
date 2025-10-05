use std::net::SocketAddr;


use remote_file_system::client::Client;

pub struct Agent {
    client: Client,
    id: u32,
}

impl Agent {
    pub fn new(id: u32, server_addr: &SocketAddr, addr: SocketAddr) -> Self {
        let client = Client::new(*server_addr, addr);
        Self {client, id}
    }

    pub fn run(&self) {
        println!("Agent {} started on address {}",
            self.id,
            self.client.get_client_address()
        );
        // Example operations
        let filename = format!("file.txt");// format!("file_{}.txt", self.id);
        let fd = 0; // self.id as i32; // Using id as file descriptor for simplicity

        // Open file
        println!("Agent {} opening file {}", self.id, filename);
        match self.client.abre(fd, filename.clone()) {
            0 => println!("Agent {} opened file {}", self.id, filename),
            i => println!("Error: Agent {} failed to open file {}: {}", self.id, filename, i),
        }

        // Write to file
        let data = format!("Hello from agent {}", self.id);
        println!("Agent {} writing to file {}", self.id, filename);
        match self.client.escreve(fd, 0, &mut data.as_bytes().to_vec(), data.len()) {
            0 => println!("Agent {} wrote to file {}", self.id, filename),
            i => println!("Error: Agent {} failed to write to file {}: {}", self.id, filename, i),
        }

        // Read from file
        for _ in 0..2 {
            let mut buffer = vec![];
            println!("Agent {} reading from file {}", self.id, filename);
            match self.client.le(fd, 0, &mut buffer, data.len()) {
                -1 => println!("Error: Agent {} failed to read from file {}", self.id, filename),
                size => {
                    let msg = String::from_utf8_lossy(&buffer[..size as usize]);
                    println!("Agent {} read data: {:?}", self.id, msg);
                },
            }
        }

        // Close file
        println!("Agent {} closing file {}", self.id, filename);
        match self.client.fecha(fd) {
            0 => println!("Agent {} closed file {}", self.id, filename),
            i => println!("Error: Agent {} failed to close file {}: {}", self.id, filename, i),
        }
    }
    
}
