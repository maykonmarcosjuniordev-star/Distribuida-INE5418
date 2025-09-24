use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, UdpSocket};
// use std::sync::mpsc::{Sender, self};

use crate::file_manager::FileManager;

pub struct Server {
    files: FileManager,
    address: SocketAddr,
    socket: TcpStream,
    socket_udp: UdpSocket,
    current_client: IpAddr,
    users: HashMap<u32, Vec<IpAddr>>, // Map of file descriptors, in each position are the clients using it.
    buffer: Vec<u8>,
}

pub struct Request {
    request_type: u8,
    file_read_or_write: u8,
    file_id: u8,
    posicao: u32,
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
        Self {files, address, socket, socket_udp}
    }

    pub fn get_address(&self) -> SocketAddr {
        self.address
    }

    pub fn abre(&self, nome_arquivo: String, op: u8) -> i32 {
        let mut descritor_arquivo;
        match self.files.abre(descritor_arquivo, nome_arquivo) {
            0 => { // TODO diferença entre write e read
                if op == 1 {
                    users[descritor_arquivo].push(current_client); // write...?
                } else {
                    users[descritor_arquivo].push(current_client); // read...?
                }
                0
            }
            Some(i) => i
        }
    }

    pub fn le(&self, descritor_arquivo: i32, posicao: u64, tamanho: usize) {
        self.files.le(descritor_arquivo, posicao, self.buffer, tamanho);
        self.send(self.buffer, self.src);
    }

    pub fn escreve(&self, descritor_arquivo: u32, posicao: u64, buffer: &mut Vec<u8>, tamanho: usize) {
        self.files.escreve(descritor_arquivo, posicao, buffer, tamanho);
        
        self.send(buffer, self.src); // Precisa avisar o cliente que a escrita deu certo?
        // TODO Atualizar todos os clientes que estão com esse arquivo aberto, que ele foi alterado.
        Response {response_type: 1}
    }

    pub fn fecha(&self, descritor_arquivo: u32) {
        self.files.fecha(descritor_arquivo)

        users[descritor_arquivo].retain(|&x| x != self.src); // remove(?) cliente da lista
    }

    pub fn send(buffer: Vec<u8>, src: IpAddr) {
        socket.send_to(&buffer, src);
    }

    pub fn desserialize(buffer: Vec<u8>) -> Request {
        Request {
            request_type: buffer[0],
            file_read_or_write: buffer[1],
            file_id: buffer[2],
            posicao: u32::from_be_bytes(buffer[3..6].try_into().unwrap()),
            size: u32::from_be_bytes(buffer[7..10].try_into().unwrap()),
            data: String::from_utf8(buffer[11..]).unwrap()
        }
    }

    pub fn main(self) {
        let mut bufferSocket = [0; 1024];
        while (let Ok((amt, src)) = self.socket.recv_from(&mut bufferSocket)) {
            self.current_client = src;
            let mut bufferVect = vec![];
            let requisito = self.desserialize(bufferSocket);

            for i in 0..amt {
                bufferVect.push(bufferSocket[i]);
            }

            let requisito = self.desserialize(buffer);

            match requisito.request_type {
                0 => self.abre(requisito.data, requisito.file_read_or_write),
                1 => self.le(requisito.file_id, requisito.posicao, requisito.size),
                2 => self.escreve(requisito.file_id, requisito.posicao, requisito.data, requisito.size),
                3 => self.fecha(requisito.file_id),
                _ => {},
            };
        }
    }
}
