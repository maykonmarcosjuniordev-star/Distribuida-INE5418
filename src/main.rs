use remote_file_system::server::Server;
use remote_file_system::client::Client;

fn main() {
    let server = Server::new();
    let _ = Client::new(server);
}
