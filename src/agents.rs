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
            0 => {
                println!("Agent {} opened file {}", self.id, filename);
            },
            i => {
                println!("Error: Agent {} failed to open file {}: {} --> Stopping", self.id, filename, i);
                return;
            },
        }

        // Write to file
        let data = format!("Hello from agent {}\n", self.id);
        println!("Agent {} writing {} to file {}", self.id, data, filename);
        let buffer = data.as_bytes().to_vec();
        let tamanho = buffer.len();
        let mut posicao = 0;
        for _ in 0..2 {
            match self.client.escreve(fd, posicao, &buffer, tamanho) {
                0 => {
                    println!("Agent {} wrote {} to file {}", self.id, data, filename);
                    posicao += tamanho as u64;
                },
                i => {
                    println!("Error: Agent {} failed to write to file {}: {}, reopening", self.id, filename, i);
                    self.client.abre(fd, filename.clone());
                },
            }
        }
        
        // Read from file
        for _ in 0..2 {
            let mut buffer = vec![];
            println!("Agent {} reading from file {}", self.id, filename);
            match self.client.le(fd, 0, &mut buffer, tamanho) {
                -1 => {
                    println!("Error: Agent {} failed to read from file {}", self.id, filename);
                    self.client.abre(fd, filename.clone());
                },
                size => {
                    let msg = String::from_utf8_lossy(&buffer[..size as usize]);
                    println!("Agent {} read {} bytes: {:?}", self.id, size, msg);
                    posicao += tamanho as u64;
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
