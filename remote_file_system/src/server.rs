use std::io::{Write, Read, ErrorKind::WouldBlock};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, TcpListener};
use std::collections::HashMap;
use std::time::{Instant, Duration};

use crate::file_manager::FileManager;
use crate::protocol::{Request, Response, RequestType, ResponseType, BUFFER_SIZE};

pub struct Server {
    files: FileManager,
    address: SocketAddr,
    /// Map of file descriptors, in each position are the clients using it.
    file_watchers: HashMap<i32, Vec<u32>>, 
}

impl Server {
    pub fn new(ip_addres: &str, port: u16) -> Self {
        let files = FileManager::new();
        let ip = ip_addres.parse::<Ipv4Addr>().expect("Failed to parse IP address");
        let address = SocketAddr::new(IpAddr::V4(ip), port);
        let file_watchers = HashMap::new();
        Self {files, address, file_watchers}
    }
    
    pub fn get_address(&self) -> SocketAddr {
        self.address
    }

    fn get_client_address_from_id(&self, client_id: u32) -> SocketAddr {
        let ip = Ipv4Addr::new(127, 0, 0, 1);
        let port = 8081 + client_id as u16;
        SocketAddr::new(IpAddr::V4(ip), port)
    }

    fn is_file_open(&self, descritor_arquivo: i32) -> bool {
        if self.file_watchers.contains_key(&descritor_arquivo) {
            return true;
        }
        println!("File {} is not open by any client", descritor_arquivo);
        false
    }

    fn add_file_watcher(&mut self, descritor_arquivo: i32, client_id: u32) {
        let usr = self.file_watchers.get_mut(&descritor_arquivo);
        match usr {
            Some(u) => {
                let is_watching = u.contains(&client_id);
                if !is_watching {
                    u.push(client_id);
                }
            },
            None => {
                self.file_watchers
                    .insert(descritor_arquivo,
                            vec![client_id]
                    );
            }
        }
    }
    
    /// Envia o buffer para o endereço do cliente
    /// Creando uma stream TCP
    fn send(buffer: Vec<u8>, stream: &mut TcpStream) {
        stream.write(&buffer).expect("Failed to write to stream");
    }

    /// Chama o file manager para abrir o arquivo
    pub fn abre(&mut self, descritor_arquivo: i32,
                nome_arquivo: String, client: &mut TcpStream, client_id: u32) {
        match self.files.abre(descritor_arquivo, &nome_arquivo) {
            0 => {
                self.add_file_watcher(descritor_arquivo, client_id);
            },
            i => {
                println!("Server failed to open file {} of name {}: Error {}", descritor_arquivo, nome_arquivo, i);
                let resp = Response {response_type: ResponseType::Erro, data: i.to_be_bytes().to_vec()};
                let buffer = Response::serialize(resp);
                Self::send(buffer, client);
            },
        }
    }

    /// Chama o file manager para ler o arquivo e envia o buffer para o cliente
    pub fn le(&mut self, descritor_arquivo: i32, posicao: u64,
                tamanho: usize, client: &mut TcpStream, client_id: u32) {
        self.is_file_open(descritor_arquivo);

        let mut buffer: Vec<u8> = vec![0; BUFFER_SIZE];
        match self.files.le(descritor_arquivo, posicao, &mut buffer, tamanho) {
            -1 => {
                println!("Server failed to read the file {}: Error {}", descritor_arquivo, -1);
                let i: i32 = -1;
                let resp = Response {response_type: ResponseType::Erro, data: i.to_be_bytes().to_vec()};
                let buffer = Response::serialize(resp);
                Self::send(buffer, client);
            },
            i => {
                println!("Server read {} bytes from file {}", i, descritor_arquivo);

                self.add_file_watcher(descritor_arquivo, client_id);

                let resp = Response {response_type: ResponseType::Ok, data: buffer[0..i as usize].to_vec()};
                let buffer = Response::serialize(resp);
                Self::send(buffer, client);
            },
        }
    }

    /// Chama o file manager para escrever no arquivo.
    /// Invalida os caches dos outros clientes que possuem o arquivo aberto
    pub fn escreve(&mut self, descritor_arquivo: i32, posicao: u64,
                    buffer: &mut Vec<u8>, tamanho: usize,
                    client: &mut TcpStream, client_id: u32) {
        self.is_file_open(descritor_arquivo);

        match self.files.escreve(descritor_arquivo, posicao, buffer, tamanho) {
            -1 => {
                println!("Server failed to write the file {}", descritor_arquivo);
                let resp = Response {response_type: ResponseType::Erro, data: [0u8; 0].to_vec()};
                let buffer = Response::serialize(resp);
                Self::send(buffer, client);
            },
            i => {
                println!("Server wrote {} bytes to file {}", i, descritor_arquivo);
                let usrs = self.file_watchers
                    .get(&descritor_arquivo)
                    .expect("shouldn't happen");
                println!("Notifying other clients to invalidate their cache");
                for usr in usrs {
                    if *usr != client_id {
                        println!("-> Notifying client {} to invalidate cache of file {}", usr, descritor_arquivo);
                        let response = Response::create_atualiza_cache(descritor_arquivo, posicao, tamanho);
                        let buffer = Response::serialize(response);
                        let user_address = self.get_client_address_from_id(*usr);
                        if let Ok(mut stream) = TcpStream::connect(user_address) {
                            Self::send(buffer, &mut stream);
                        } else {
                            println!("-> Server failed to connect to client {}", usr);
                        }
                    }
                }

                println!("Removing other clients from the list of users of the file, only keeping the client that wrote");
                // mantém apenas o cliente que fez a escrita na lista de usuários
                self.file_watchers.clear();
                self.add_file_watcher(descritor_arquivo, client_id);
                let resp = Response {response_type: ResponseType::Ok, data: i.to_be_bytes().to_vec()};
                let buffer = Response::serialize(resp);
                Self::send(buffer, client);
            },
        }
    }

    /// Chama o file manager para fechar o arquivo
    /// Remove o cliente da lista de usuários do arquivo
    pub fn fecha(&mut self, descritor_arquivo: i32, client: &mut TcpStream, client_id: u32) {
        self.is_file_open(descritor_arquivo);
        match self.files.fecha(descritor_arquivo) {
            0 => {
                println!("Server closed file {}", descritor_arquivo);
                self.file_watchers
                    .get_mut(&descritor_arquivo)
                    .expect("File not found")
                    .retain(|&x| x != client_id);

            },
            i => {
                println!("Server failed to close file {}: Error {}", descritor_arquivo, i);
                let resp = Response {response_type: ResponseType::Erro, data: i.to_be_bytes().to_vec()};
                let buffer = Response::serialize(resp);
                Self::send(buffer, client);
            },
        }

    }

    pub fn run(&mut self) {
        println!("Server listening on {}", self.address);
        let listener = TcpListener::bind(self.address).expect("Failed to bind server address");
        // accept connections and process them serially
        // with a 1000 ms timeout
        // listener.set_nonblocking(true).expect("Failed to set non-blocking");
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
                break;
                }
                // avoid busy-looping
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
            Err(e) => {
                // an actual error occurred while accepting
                Err(e)
            }
            };
            match stream {
                Ok(mut stream) => {
                    let current_client = stream.peer_addr().expect("Failed to get client address");
                    println!("New connection on Server: {}", current_client);
                    let mut buffer_socket = vec![0; 1024];
                    let amt = stream.read(&mut buffer_socket).expect("Failed to read from socket");
                    println!("Server Received {} bytes from {}", amt, current_client);
                    let mut buffer_vect = vec![];
                    let request = Request::desserialize(&buffer_socket);

                    println!("Request is from client {}", request.client_id);
        
                    for i in 0..amt {
                        buffer_vect.push(buffer_socket[i].clone());
                    }
        
                    match request.request_type {
                        RequestType::Abre => {
                            println!("Server opening file {}", request.descritor_arquivo);
                            self.abre(
                                request.descritor_arquivo, 
                                String::from_utf8(request.data).expect("Failed to convert data to string"), 
                                &mut stream,
                                request.client_id);
                        },
                        RequestType::Le => {
                            println!("Server reading file {}", request.descritor_arquivo);
                            self.le(
                                request.descritor_arquivo, 
                                request.posicao, 
                                request.size as usize, 
                                &mut stream,
                                request.client_id);
                        },
                        RequestType::Escreve => {
                            println!("Server writing file {}", request.descritor_arquivo);
                            let mut buffer = request.data;
                            self.escreve(
                                request.descritor_arquivo, 
                                request.posicao, 
                                &mut buffer, 
                                request.size as usize, 
                                &mut stream,
                                request.client_id);
                        },
                        RequestType::Fecha => {
                            println!("Server closing file {}", request.descritor_arquivo);
                            self.fecha(
                                request.descritor_arquivo, 
                                &mut stream,
                                request.client_id);
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
