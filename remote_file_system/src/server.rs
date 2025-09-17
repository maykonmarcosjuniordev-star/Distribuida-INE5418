use std::net::{IpAddr, Ipv4Addr, SocketAddr};
// use std::sync::mpsc::{Sender, self};

use crate::file_manager::FileManager;

pub struct Server {
    files: FileManager,
    address: SocketAddr
}

impl Server {
    pub fn new(ip_addres: &str, port: u16) -> Self {
        let files = FileManager::new();
        let ip = ip_addres.parse::<Ipv4Addr>().unwrap();
        let address = SocketAddr::new(IpAddr::V4(ip), port);
        Self { files, address }
    }

    pub fn get_address(&self) -> SocketAddr {
        self.address
    }

    pub fn abre(&self, descritor_arquivo: i32, nome_arquivo: String) -> i32 {
        self.files.abre(descritor_arquivo, nome_arquivo)
    }
}
