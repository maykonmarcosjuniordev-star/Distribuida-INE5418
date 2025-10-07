use std::io::{Write, Read};
use std::net::{UdpSocket, IpAddr, Ipv4Addr, SocketAddr, TcpStream, TcpListener};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::vec;

use crate::file_manager::FileManager;
use crate::protocol::{Request, Response, RequestType, StandardResponseFactory, ResponseFactory, BUFFER_SIZE};

struct RateLimiter {
    pub number_of_requests: u32,
    pub first_request: Duration,
}

pub struct Server {
    files: FileManager,
    address: SocketAddr,
    /// Map of file descriptors, in each position are the clients using it.
    file_watchers: Mutex<HashMap<i32, Vec<SocketAddr>>>,
    response_factory: StandardResponseFactory,
    throttle: Mutex<HashMap<SocketAddr, RateLimiter>>,

}

#[macro_export]
macro_rules! server_log {
    // Rule for the case where arguments are provided (e.g., simple_log!("Error: {}", e))
    ($fmt:literal $(, $args:expr)*) => {
        eprintln!(
            // Combine the fixed prefix and the user's format string
            concat!("[SERVER]: ", $fmt), 
            $($args),*
        )
    };
    // Rule for the case where NO arguments are provided (e.g., simple_log!("Status ok"))
    ($fmt:literal) => {
        eprintln!(concat!("[SERVER]: ", $fmt))
    };
}

impl Server {
    pub fn new(ip_addres: &str, port: u16) -> Self {
        let files = FileManager::new();
        let ip = ip_addres.parse::<Ipv4Addr>().expect("Failed to parse IP address");
        let address = SocketAddr::new(IpAddr::V4(ip), port);
        let file_watchers = Mutex::new(HashMap::new());
        let response_factory = StandardResponseFactory;
        let throttle = Mutex::new(HashMap::new());
        Self {files, address, file_watchers, response_factory, throttle}
    }
    
    pub fn get_address(&self) -> SocketAddr {
        self.address
    }
    
    /// Envia o buffer para o endereço do cliente
    /// Creando uma stream TCP
    fn send(response: Response, stream: &mut TcpStream) {
        let buffer = Response::serialize(&response);
        stream.write(&buffer).expect("Failed to write to stream");
    }

    /// Envia mensagens de invalidação de cache para o cliente
    /// usando Sockets UDP para permitir que haja buffer de mensagens
    fn send_warning(&self, response: Response, descritor_arquivo: i32) {
        if let Ok(watchers) = self.file_watchers.lock() {
            let usrs = watchers
                .get(&descritor_arquivo)
                .expect("File not found");
            let socket = UdpSocket::bind(self.address)
                .expect("Failed to bind UDP socket on server");
            for usr in usrs {
                let buffer = Response::serialize(&response);
                match socket.send_to(&buffer, usr) {
                    Ok(size) => server_log!("Sent cache invalidation for file {} to {} with {} bytes", descritor_arquivo, usr, size),
                    Err(e) => server_log!("Failed to send cache invalidation to {}: {}", usr, e),
                }
            }
        }
    }

    /// Chama o file manager para abrir o arquivo
    pub fn abre(&self, request: Request, client: &mut TcpStream) {
        let nome_arquivo = String::from_utf8(request.data).expect("Failed to convert data to string");
        match self.files.abre(request.descritor_arquivo, &nome_arquivo) {
            0 => {
                if let Ok(mut usr) = self.file_watchers.lock() {
                    match usr.get_mut(&request.descritor_arquivo) {
                        Some(u) => {
                            let client_address = client.peer_addr().expect("Failed to get client address");
                            if !u.contains(&client_address) {
                                u.push(client_address);
                            }
                        },
                        None => {
                            usr
                                .insert(request.descritor_arquivo,
                                    vec![client.peer_addr()
                                        .expect("Failed to get client address")]
                                );
                        }
                    }
                }
            },
            i => {
                server_log!("Server failed to open file {} of name {}: Error {}", request.descritor_arquivo, nome_arquivo, i);
                let response = self.response_factory.create_error_response(-1);
                Self::send(response, client);
            },
        }
    }

    fn has_open(&self, request: &Request, client: &mut TcpStream) -> bool {
        // verify if the file is being watched, by the client, if not, return error
        if let Ok(watchers) = self.file_watchers.lock() {
            if let Some(usrs) = watchers.get(&request.descritor_arquivo) {
                let addr = client.peer_addr().expect("Failed to get client address");
                if !usrs.contains(&addr) {
                    server_log!("Client {} is not watching file {}", addr, request.descritor_arquivo);
                    let response = self.response_factory.create_error_response(-1);
                    Self::send(response, client);
                    return false;
                }
            }
        }
        return true;
    }
    
    /// Chama o file manager para ler o arquivo e envia o buffer para o cliente
    pub fn le(&self, request: Request, client: &mut TcpStream) {
        if !self.has_open(&request, client) {
            return;
        }
        let mut buffer: Vec<u8> = vec![0; BUFFER_SIZE];
        match self.files.le(request.descritor_arquivo, request.posicao, &mut buffer, request.tamanho as usize) {
            -1 => {
                server_log!("Server failed to read the file {}: Error {}", request.descritor_arquivo, -1);
                let response = self.response_factory.create_error_response(-1);
                Self::send(response, client);
            },
            i => {
                let response = self.response_factory.create_success_response(buffer[0..i as usize].to_vec());
                Self::send(response, client);
            },
        }
    }

    /// Chama o file manager para escrever no arquivo.
    /// Invalida os caches dos outros clientes que possuem o arquivo aberto
    pub fn escreve(&self, request: Request, client: &mut TcpStream) {
        if !self.has_open(&request, client) {
            return;
        }
        let mut buffer = request.data;
        match self.files.escreve(request.descritor_arquivo, request.posicao, &mut buffer, request.tamanho as usize) {
            -1 => {
                server_log!("Server failed to write the file {}", request.descritor_arquivo);
                let response = self.response_factory.create_error_response(-1);
                Self::send(response, client);
            },
            i => {
                server_log!("Sending cache invalidation for file {} from position {} for {} bytes", request.descritor_arquivo, request.posicao, request.tamanho);
                let response = self.response_factory
                    .create_cache_invalidation(request.descritor_arquivo,
                        request.posicao,
                        request.tamanho as usize
                    );
                self.send_warning(response, request.descritor_arquivo);
                // let addr = client
                //     .peer_addr()
                //     .expect("Failed to get client address");
                // // mantém apenas o cliente que fez a escrita na lista de usuários
                // self.file_watchers
                //     .lock()
                //     .expect("Failed to lock file watchers")
                //     .get_mut(&request.descritor_arquivo)
                //     .expect("File not found")
                //     .retain(|&x| x == addr);
                let response = self.response_factory.create_success_response(i.to_be_bytes().to_vec());
                Self::send(response, client);
            },
        }
    }

    /// Chama o file manager para fechar o arquivo
    /// Remove o cliente da lista de usuários do arquivo
    pub fn fecha(&self, request: Request, client: &mut TcpStream) {
        if !self.has_open(&request, client) {
            return;
        }
        if let Ok(mut watchers) = self.file_watchers.lock() {
            if let Some(usrs) = watchers.get_mut(&request.descritor_arquivo) {
                if usrs.len() > 1 {
                    // else, just remove the client from the list
                    let addr = client.peer_addr().expect("Failed to get client address");
                    usrs.retain(|&x| x != addr);
                    // if there are still users, do not close the file
                    return;
                }
            }
        }
        // if the client is the last one, remove the entry
        match self.files.fecha(request.descritor_arquivo) {
            0 => {
                // remove the entry from the watchers
                if let Ok(mut watchers) = self.file_watchers.lock() {
                    watchers.remove(&request.descritor_arquivo);
                }
            },
            i => {
                server_log!("Server failed to close file {}: Error {}", request.descritor_arquivo, i);
                let response = self.response_factory.create_error_response(-1);
                Self::send(response, client);
            },
        }

    }

    pub fn run(&self) {
        server_log!("Server listening on {}", self.address);
        let listener = TcpListener::bind(self.address).expect("Failed to bind server address");
        // accept connections and process them serially
        let start_time = Instant::now();
        loop {
            match listener.accept() {
                Ok((mut stream, current_client)) => {
                    if let Ok(mut throttle) = self.throttle.lock() {
                        let entry = throttle.entry(current_client).or_insert(RateLimiter {
                            number_of_requests: 0,
                            first_request: start_time.elapsed(),
                        });
                        entry.number_of_requests += 1;
                        let elapsed = start_time.elapsed() - entry.first_request;
                        if elapsed.as_secs() >= 1 {
                            // reset the counter
                            entry.number_of_requests = 1;
                            entry.first_request = start_time.elapsed();
                        } else if entry.number_of_requests > 5 {
                            println!("Throttling client {}: too many requests", current_client);
                            let response = self.response_factory.create_error_response(-1);
                            Self::send(response, &mut stream);
                            continue;
                        }                        
                    }
                    let mut buffer_socket = vec![0; BUFFER_SIZE];
                    let amt = stream.read(&mut buffer_socket).expect("Failed to read from socket");
                    let request = Request::desserialize(&buffer_socket);
                    
                    match request.request_type {
                        RequestType::Abre => {
                            server_log!("Server Received a Open Request with {} bytes from {}", amt, current_client);
                            self.abre(request, &mut stream);
                        },
                        RequestType::Le => {
                            server_log!("Server Received a Read Request with {} bytes from {}", amt, current_client);
                            self.le(request, &mut stream);
                        },
                        RequestType::Escreve => {
                            server_log!("Server Received a Write Request with {} bytes from {}", amt, current_client);
                            self.escreve(request, &mut stream);
                        },
                        RequestType::Fecha => {
                            server_log!("Server Received a Close Request with {} bytes from {}", amt, current_client);
                            self.fecha(request, &mut stream);
                        }
                    };
                }
                Err(e) => {
                    server_log!("Error Receiving Connections: {}", e);
                }
            }
        }
    }
}
