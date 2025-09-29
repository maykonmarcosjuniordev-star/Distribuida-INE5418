use std::io::{Write, Read};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, TcpListener, UdpSocket};
use std::collections::HashMap;

use crate::file_manager::FileManager;

pub struct Server {
    files: FileManager,
    address: SocketAddr,
    socket: TcpStream,
    socket_udp: UdpSocket,
    // Map of file descriptors, in each position are the clients using it.
    users: HashMap<u32, Vec<SocketAddr>>, 
}

pub struct Request {
    request_type: u8,
    descritor_arquivo: u32,
    posicao: u64,
    size: u32,
    data: String,
}

pub struct Response {
    response_type: u8, // 1 -> Atualiza cache dos clientes devido a write
    data: String,
}

impl Server {
    pub fn new(ip_addres: &str, port: u16) -> Self {
        let files = FileManager::new();
        let ip = ip_addres.parse::<Ipv4Addr>().unwrap();
        let address = SocketAddr::new(IpAddr::V4(ip), port);
        let socket = TcpStream::connect(address).unwrap();
        let socket_udp = UdpSocket::bind(address).unwrap();
        let users = HashMap::new();
        Self {files, address, socket, socket_udp, users}
    }

    pub fn get_address(&self) -> SocketAddr {
        self.address
    }

    /// Chama o file manager para abrir o arquivo
    pub fn abre(&mut self, descritor_arquivo: u32,
                nome_arquivo: String, client: SocketAddr) -> i32 {
        match self.files.abre(descritor_arquivo, nome_arquivo) {
            0 => {
                let usr = self.users
                    .get_mut(&descritor_arquivo)
                    .expect("shouldn't happen");
                usr.push(client);
                0
            }
            i => {i}
        }
    }

    /// Chama o file manager para ler o arquivo e envia o buffer para o cliente
    pub fn le(&self, descritor_arquivo: u32, posicao: u64,
                tamanho: usize, client: SocketAddr) {
        let mut buffer: Vec<u8> = vec![0; tamanho];
        // TODO: Deveria verificar se o cliente tem o arquivo aberto
        // TODO: Deveria verificar se o arquivo não está sendo escrito por outro cliente
        self.files.le(descritor_arquivo, posicao, &mut buffer, tamanho);
        self.send(buffer, client);
    }

    /// Chama o file manager para escrever no arquivo.
    /// Invalida os caches dos outros clientes que possuem o arquivo aberto
    pub fn escreve(&self, descritor_arquivo: u32, posicao: u64,
                buffer: &mut Vec<u8>, tamanho: usize, client: SocketAddr) -> i32 {
        let out = self.files.escreve(descritor_arquivo, posicao, buffer, tamanho);
        // TODO: Invalida os caches dos outros clientes que possuem o arquivo aberto
        let usrs = self.users
            .get(&descritor_arquivo)
            .expect("shouldn't happen");
        return out;
    }

    /// Chama o file manager para fechar o arquivo
    /// Remove o cliente da lista de usuários do arquivo
    pub fn fecha(&mut self, descritor_arquivo: u32, client: SocketAddr) -> i32 {
        let out = self.files.fecha(descritor_arquivo);

        // TODO: Não sei se isso funciona
        self.users.get_mut(&descritor_arquivo).expect("File not found")
                .retain(|&x| x != client); // remove(?) cliente da lista
        return out;
    }

    /// Envia o buffer para o endereço do cliente
    /// Utiliza o Socket TCP
    pub fn send(&self, buffer: Vec<u8>, src: SocketAddr) {
        let mut stream = TcpStream::connect(src).unwrap();
        stream.write(&buffer).unwrap();
    }

    /// Desserializa o buffer recebido do cliente
    pub fn desserialize(buffer: &Vec<u8>) -> Request {
        // Expected layout:
        // [0]                 -> request_type (1 byte)
        // [1..5]              -> descritor_arquivo (4 bytes, big-endian)
        // [5..9]              -> posicao (4 bytes, big-endian)
        // [9..13]             -> size (4 bytes, big-endian)
        // [13..]              -> data (remaining bytes)
        Request {
            request_type: buffer[0],
            descritor_arquivo: u32::from_be_bytes(buffer[1..5].try_into().unwrap()),
            posicao: u64::from_be_bytes(buffer[5..13].try_into().unwrap()),
            size: u32::from_be_bytes(buffer[13..17].try_into().unwrap()),
            data: String::from_utf8(buffer[17..].to_vec()).unwrap(),
        }
    }

    pub fn main(&mut self) {        
        // reads from socket
        let listener = TcpListener::bind(self.address).unwrap();
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let current_client = stream.peer_addr().unwrap();
                    println!("New connection: {}", current_client);
                    let mut buffer_socket = vec![0; 1024];
                    let amt = stream.read(&mut buffer_socket).unwrap();
                    println!("Received {} bytes from {}", amt, current_client);
                    let mut bufferVect = vec![];
                    let requisito = Self::desserialize(&buffer_socket);
        
                    for i in 0..amt {
                        bufferVect.push(buffer_socket[i].clone());
                    }
        
                    match requisito.request_type {
                        0 => {
                            self.abre(requisito.descritor_arquivo, requisito.data, current_client);
                        },
                        1 => {
                            self.le(requisito.descritor_arquivo, requisito.posicao, requisito.size as usize, current_client);
                        },
                        2 => {
                            let mut buffer = requisito.data.as_bytes().to_vec();
                            self.escreve(requisito.descritor_arquivo, requisito.posicao, &mut buffer, requisito.size as usize, current_client);
                        },
                        3 => {
                            self.fecha(requisito.descritor_arquivo, current_client);
                        },
                        _ => {
                            println!("Invalid request type");
                        },
                    };
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }
    }
}
