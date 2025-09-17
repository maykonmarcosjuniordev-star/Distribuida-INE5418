use remote_file_system::server::Server;
use remote_file_system::client::Client;

fn main() {
    let server = Server::new("127.0.0.1", 8080);
    let _ = Client::new(server.get_address());
}
