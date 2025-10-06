use std::io::{Write, Read, ErrorKind::WouldBlock};
use std::net::{UdpSocket, IpAddr, Ipv4Addr, SocketAddr, TcpStream, TcpListener};
use std::collections::HashMap;
use std::time::{Instant, Duration};
use std::sync::Mutex;
use std::vec;

use crate::file_manager::FileManager;
use crate::protocol::{Request, Response, RequestType, StandardResponseFactory, ResponseFactory, BUFFER_SIZE};

pub struct Server {
    files: FileManager,
    address: SocketAddr,
    /// Map of file descriptors, in each position are the clients using it.
    file_watchers: Mutex<HashMap<i32, Vec<SocketAddr>>>,
    response_factory: StandardResponseFactory,
}

impl Server {
    pub fn new(ip_addres: &str, port: u16) -> Self {
        let files = FileManager::new();
        let ip = ip_addres.parse::<Ipv4Addr>().expect("Failed to parse IP address");
        let address = SocketAddr::new(IpAddr::V4(ip), port);
        let file_watchers = Mutex::new(HashMap::new());
        let response_factory = StandardResponseFactory;
        Self {files, address, file_watchers, response_factory}
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
                    Ok(size) => println!("Sent cache invalidation to {}: {} bytes", usr, size),
                    Err(e) => println!("Failed to send cache invalidation to {}: {}", usr, e),
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
                            u.push(client
                                .peer_addr()
                                .expect("Failed to get client address"));
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
                println!("Server failed to open file {} of name {}: Error {}", request.descritor_arquivo, nome_arquivo, i);
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
                    println!("Client {} is not watching file {}", addr, request.descritor_arquivo);
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
                println!("Server failed to read the file {}: Error {}", request.descritor_arquivo, -1);
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
                println!("Server failed to write the file {}", request.descritor_arquivo);
                let response = self.response_factory.create_error_response(-1);
                Self::send(response, client);
            },
            i => {
                let response = self.response_factory
                    .create_cache_invalidation(request.descritor_arquivo,
                        request.posicao,
                        request.tamanho as usize
                    );
                self.send_warning(response, request.descritor_arquivo);
                let addr = client
                    .peer_addr()
                    .expect("Failed to get client address");
                // mantém apenas o cliente que fez a escrita na lista de usuários
                self.file_watchers
                    .lock()
                    .expect("Failed to lock file watchers")
                    .get_mut(&request.descritor_arquivo)
                    .expect("File not found")
                    .retain(|&x| x == addr);
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
        match self.files.fecha(request.descritor_arquivo) {
            0 => {
                let addr = client.peer_addr().expect("Failed to get client address");
                self.file_watchers
                    .lock()
                    .expect("Failed to lock file watchers")
                    .get_mut(&request.descritor_arquivo)
                    .expect("File not found")
                    .retain(|&x| x != addr);

            },
            i => {
                println!("Server failed to close file {}: Error {}", request.descritor_arquivo, i);
                let response = self.response_factory.create_error_response(-1);
                Self::send(response, client);
            },
        }

    }

    pub fn run(&self) {
        println!("Server listening on {}", self.address);
        let listener = TcpListener::bind(self.address).expect("Failed to bind server address");
        // accept connections and process them serially
        // with a 1000 ms timeout
        listener.set_nonblocking(true).expect("Failed to set non-blocking");
        let timeout = Duration::from_millis(1000);
        let mut last_activity = Instant::now();
        loop {
            // try to accept a connection (non-blocking)
            let stream = match listener.accept() {
            Ok((s, _addr)) => {
                // got a connection, reset inactivity timer and provide Ok(TcpStream)
                last_activity = Instant::now();
                Ok(s)
            }
            Err(ref e) if e.kind() == WouldBlock => {
                // no connection available right now
                if last_activity.elapsed() >= timeout {
                    // no activity for the timeout period -> break the loop
                    println!("Server timed out due to inactivity.");
                    break;
                }
                // avoid busy-looping
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
            Err(e) => {
                // an actual error occurred while accepting
                println!("Error accepting connection: {}", e);
                Err(e)
            }
            };
            match stream {
                Ok(mut stream) => {
                    let current_client = stream.peer_addr().expect("Failed to get client address");
                    let mut buffer_socket = vec![0; 1024];
                    let amt = stream.read(&mut buffer_socket).expect("Failed to read from socket");
                    println!("Server Received {} bytes from {}", amt, current_client);
                    let mut buffer_vect = vec![];
                    let request = Request::desserialize(&buffer_socket);
        
                    for i in 0..amt {
                        buffer_vect.push(buffer_socket[i].clone());
                    }
        
                    match request.request_type {
                        RequestType::Abre => {
                            self.abre(request, &mut stream);
                        },
                        RequestType::Le => {
                            self.le(request, &mut stream);
                        },
                        RequestType::Escreve => {
                            self.escreve(request, &mut stream);
                        },
                        RequestType::Fecha => {
                            self.fecha(request, &mut stream);
                        }
                    };
                }
                Err(e) => {
                    println!("Error Receiving Connections: {}", e);
                }
            }
        }
    }
}
