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
    file_watchers: HashMap<i32, Vec<SocketAddr>>, 
}

impl Server {
    pub fn new(ip_addres: &str, port: u16) -> Self {
        let files = FileManager::new();
        let ip = ip_addres.parse::<Ipv4Addr>().unwrap();
        let address = SocketAddr::new(IpAddr::V4(ip), port);
        let file_watchers = HashMap::new();
        Self {files, address, file_watchers}
    }
    
    pub fn get_address(&self) -> SocketAddr {
        self.address
    }
    
    /// Envia o buffer para o endereço do cliente
    /// Creando uma stream TCP
    fn send(buffer: Vec<u8>, stream: &mut TcpStream) {
        stream.write(&buffer).unwrap();
    }

    /// Chama o file manager para abrir o arquivo
    pub fn abre(&mut self, descritor_arquivo: i32,
                nome_arquivo: String, client: &mut TcpStream) {
        match self.files.abre(descritor_arquivo, &nome_arquivo) {
            0 => {
                let usr = self.file_watchers.get_mut(&descritor_arquivo);
                match usr {
                    Some(u) => u.push(client.peer_addr().unwrap()),
                    None => {
                        self.file_watchers.insert(descritor_arquivo, vec![client.peer_addr().unwrap()]);
                    }
                }
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
    pub fn le(&self, descritor_arquivo: i32, posicao: u64,
                tamanho: usize, client: &mut TcpStream) {
        // Verifica se o arquivo está aberto por algum cliente
        if !self.file_watchers.contains_key(&descritor_arquivo) {
            println!("File not opened by any client");
            return;
        }
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
                    client: &mut TcpStream) {
        match self.files.escreve(descritor_arquivo, posicao, buffer, tamanho) {
            -1 => {
                println!("Server failed to write the file {}", descritor_arquivo);
                let resp = Response {response_type: ResponseType::Erro, data: [0u8; 0].to_vec()};
                let buffer = Response::serialize(resp);
                Self::send(buffer, client);
            },
            i => {
                let usrs = self.file_watchers
                    .get(&descritor_arquivo)
                    .expect("shouldn't happen");
                let addr = client.peer_addr().unwrap();
                for usr in usrs {
                    if *usr != addr {
                        let response = Response::create_atualiza_cache(descritor_arquivo, posicao, tamanho);
                        let buffer = Response::serialize(response);
                        let mut stream = TcpStream::connect(usr).unwrap();
                        Self::send(buffer, &mut stream);
                    }
                }
                // mantém apenas o cliente que fez a escrita na lista de usuários
                self.file_watchers
                    .get_mut(&descritor_arquivo)
                    .expect("File not found")
                    .retain(|&x| x == addr);
                let resp = Response {response_type: ResponseType::Ok, data: i.to_be_bytes().to_vec()};
                let buffer = Response::serialize(resp);
                Self::send(buffer, client);
            },
        }
    }

    /// Chama o file manager para fechar o arquivo
    /// Remove o cliente da lista de usuários do arquivo
    pub fn fecha(&mut self, descritor_arquivo: i32, client: &mut TcpStream) {
        match self.files.fecha(descritor_arquivo) {
            0 => {
                let addr = client.peer_addr().unwrap();
                self.file_watchers
                    .get_mut(&descritor_arquivo)
                    .expect("File not found")
                    .retain(|&x| x != addr);

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
                    let current_client = stream.peer_addr().unwrap();
                    println!("New connection on Server: {}", current_client);
                    let mut buffer_socket = vec![0; 1024];
                    let amt = stream.read(&mut buffer_socket).unwrap();
                    println!("Server Received {} bytes from {}", amt, current_client);
                    let mut buffer_vect = vec![];
                    let requisito = Request::desserialize(&buffer_socket);
        
                    for i in 0..amt {
                        buffer_vect.push(buffer_socket[i].clone());
                    }
        
                    match requisito.request_type {
                        RequestType::Abre => {
                            self.abre(requisito.descritor_arquivo, String::from_utf8(requisito.data).unwrap(), &mut stream);
                        },
                        RequestType::Le => {
                            self.le(requisito.descritor_arquivo, requisito.posicao, requisito.size as usize, &mut stream);
                        },
                        RequestType::Escreve => {
                            let mut buffer = requisito.data;
                            self.escreve(requisito.descritor_arquivo, requisito.posicao, &mut buffer, requisito.size as usize, &mut stream);
                        },
                        RequestType::Fecha => {
                            self.fecha(requisito.descritor_arquivo, &mut stream);
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
