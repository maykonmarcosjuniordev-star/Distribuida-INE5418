use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, UdpSocket};
// use std::sync::mpsc::{Sender, self};

use crate::file_manager::FileManager;

pub struct Server {
    files: FileManager,
    address: SocketAddr,
    socket: TcpStream,
    socket_udp: UdpSocket,
    // clientes: Vec<>,
    // buffer: Vec<>
}

pub struct Request {
    request_type: u8,
    file_op: u8,
    file_name: String,
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

    pub fn abre(&self, descritor_arquivo: i32, nome_arquivo: String) -> i32 {
        self.files.abre(descritor_arquivo, nome_arquivo)
    }

    // pub fn le(&self, descritor_arquivo: i32, posicao: u64, buffer: &mut Vec<u8>, tamanho: usize) {
    //     match self.files.le(descritor_arquivo, posicao: u64, buffer: &mut Vec<u8>, tamanho: usize) {
    //         Some(t) => 0,
    //         Err(e) => e,
    //     }
    // }

    // pub fn enviar_buffer(buffer: Vec<u8>, dest: IpAddr) {

    //     socket.send_to(buf, &dest)?;
    // }

    pub fn desserialize(buffer: Vec<u8>) -> Request {
        Request {request_type: buffer[0], file_op: buffer[1], file_name: String::from_utf8(buffer[2..]).unwrap()}
    }

    pub fn main(self) {
        let mut bufferSocket = [0; 1024];
        // self.vetor de clientes, add src
        while (let Ok((amt, src)) = self.socket.recv_from(&mut bufferSocket)) {
            let mut bufferVect = vec![];
            let requisito = self.desserialize(bufferSocket);

            for i in 0..amt {
                bufferVect.push(bufferSocket[i]);
            }

            let requisito = self.desserialize(buffer);

            match requisito.request_type {
                0 => self.abre(),
                1 => self.le(),
                2 => self.escreve(),
                3 => self.fecha(),
                _ => {},
            };


            // cliente.enviar(buffer);
        }
    }
}
