// use std::net::{IpAddr, Ipv4Addr, SocketAddr};
// use std::sync::mpsc::{Sender, self};

use crate::file_manager::FileManager;

pub struct Server {
    files: FileManager,
}

impl Server {
    pub fn new() -> Self {
        let files = FileManager {};
        Self { files }
    }

    pub fn abre(&self, descritor_arquivo: i32, nome_arquivo: String) -> i32 {
        self.files.abre(descritor_arquivo, nome_arquivo)
    }
}
