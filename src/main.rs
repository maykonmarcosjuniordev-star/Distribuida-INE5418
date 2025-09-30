// for multithreading
use std::thread;
use std::sync::{Arc, Mutex};


use remote_file_system::server::Server;
use remote_file_system::client::Client;



fn main() {
    let server = Server::new("127.0.0.1", 8080);
    let client = Client::new(server.get_address());
    let server = Arc::new(Mutex::new(server));
    let client = Arc::new(Mutex::new(client));
    let server_handle = {
        let server = Arc::clone(&server);
        thread::spawn(move || {
            let mut server = server.lock().unwrap();
            server.run();
        })
    };
    let client_handle = {
        let client = Arc::clone(&client);
        thread::spawn(move || {
            let client = client.lock().unwrap();
            client.abre(1, String::from("file.txt"));
            let mut buffer = vec![0; 13];
            for i in 0..13 {
                buffer[i] = i as u8;
            }
            client.escreve(1, 0, &mut buffer, 13);
        })
    };
    server_handle.join().unwrap();
    client_handle.join().unwrap();
}
