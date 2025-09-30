use std::io::{Write, Read};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, TcpListener};
use std::collections::HashMap;

use crate::file_manager::FileManager;
use crate::protocol::{Request, Response, RequestType, ResponseType};

pub struct Server {
    files: FileManager,
    address: SocketAddr,
    /// Map of file descriptors, in each position are the clients using it.
    file_watchers: HashMap<u32, Vec<SocketAddr>>, 
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

    /// Chama o file manager para abrir o arquivo
    pub fn abre(&mut self, descritor_arquivo: u32,
                nome_arquivo: String, client: SocketAddr) -> i32 {
        match self.files.abre(descritor_arquivo, nome_arquivo) {
            0 => {
                let usr = self.file_watchers.get_mut(&descritor_arquivo);
                match usr {
                    Some(u) => u.push(client),
                    None => {
                        self.file_watchers.insert(descritor_arquivo, vec![client]);
                    }
                }
                0
            }
            i => {i}
        }
    }

    /// Chama o file manager para ler o arquivo e envia o buffer para o cliente
    pub fn le(&self, descritor_arquivo: u32, posicao: u64,
                tamanho: usize, client: SocketAddr) {
        let mut buffer: Vec<u8> = vec![0; tamanho];
        // Verifica se o arquivo está aberto por algum cliente
        if !self.file_watchers.contains_key(&descritor_arquivo) {
            println!("File not opened by any client");
            return;
        }
        match self.files.le(descritor_arquivo, posicao, &mut buffer, tamanho) {
            0 => {
                self.send(buffer, client);
            },
            i => {
                println!("Error reading file: {}", i);
                return;
            }
        }
    }

    /// Chama o file manager para escrever no arquivo.
    /// Invalida os caches dos outros clientes que possuem o arquivo aberto
    pub fn escreve(&self, descritor_arquivo: u32, posicao: u64,
                    buffer: &mut Vec<u8>, tamanho: usize,
                    client: SocketAddr) -> i32 {
        match self.files.escreve(descritor_arquivo, posicao, buffer, tamanho) {
            0 => {
                let usrs = self.file_watchers
                    .get(&descritor_arquivo)
                    .expect("shouldn't happen");
                for usr in usrs {
                    if *usr != client {
                        let response = Response {
                            response_type: ResponseType::AtualizaCache,
                            data: descritor_arquivo.to_string(),
                        };
                        let buffer = Response::serialize(response);
                        self.send(buffer, *usr);
                    }
                }
                return 0;
            },
            i => {
                return i;
            },
        }
    }

    /// Chama o file manager para fechar o arquivo
    /// Remove o cliente da lista de usuários do arquivo
    pub fn fecha(&mut self, descritor_arquivo: u32, client: SocketAddr) -> i32 {
        match self.files.fecha(descritor_arquivo) {
            0 => {
                self.file_watchers
                    .get_mut(&descritor_arquivo)
                    .expect("File not found")
                    .retain(|&x| x != client);
                0
            },
            i => {
                return i;
            },
        }
    
    }

    /// Envia o buffer para o endereço do cliente
    /// Creando uma stream TCP
    pub fn send(&self, buffer: Vec<u8>, src: SocketAddr) {
        let mut stream = TcpStream::connect(src).unwrap();
        stream.write(&buffer).unwrap();
    }

    pub fn main(&mut self) {
        let listener = TcpListener::bind(self.address).unwrap();
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let current_client = stream.peer_addr().unwrap();
                    println!("New connection: {}", current_client);
                    let mut buffer_socket = vec![0; 1024];
                    let amt = stream.read(&mut buffer_socket).unwrap();
                    println!("Received {} bytes from {}", amt, current_client);
                    let mut buffer_vect = vec![];
                    let requisito = Request::desserialize(&buffer_socket);
        
                    for i in 0..amt {
                        buffer_vect.push(buffer_socket[i].clone());
                    }
        
                    match requisito.request_type {
                        RequestType::Abre => {
                            self.abre(requisito.descritor_arquivo, requisito.data, current_client);
                        },
                        RequestType::Le => {
                            self.le(requisito.descritor_arquivo, requisito.posicao, requisito.size as usize, current_client);
                        },
                        RequestType::Escreve => {
                            let mut buffer = requisito.data.as_bytes().to_vec();
                            self.escreve(requisito.descritor_arquivo, requisito.posicao, &mut buffer, requisito.size as usize, current_client);
                        },
                        RequestType::Fecha => {
                            self.fecha(requisito.descritor_arquivo, current_client);
                        }
                    };
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }
    }
}
