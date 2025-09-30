/*
Coerência de cache
As operações de leitura e de escrita podem acessar partes do arquivo (blocos) localmente ou remotamente.
Por exemplo,
    uma leitura de regiões previamente carregadas na cache local,
    retornam diretamente para a função de leitura,
    sem gerar requisições ao servidor.
Para isso, os processos solicitantes devem fazer cópias do(s) bloco(s) de interesse
    e mantê-la em uma cache local quando da escrita ou leitura desses conteúdos.
No caso de leituras e leituras sucessivas,
    se o processo tiver uma cópia válida do bloco de interesse,
    basta ler da sua cache.
No caso de escritas,
    o processo deve atualizar o valor do bloco no arquivo em questão
    e gerar uma invalidação das cópias daquele bloco em caches de outros processos.
Este procedimento é comum na implementação de mecanismos para coerência de cache
    e chama-se invalidação na escrita.
É permitida a utilização de outras estratégias para coerência de cache.
*/
use std::collections::LinkedList;
use std::net::{SocketAddr, TcpStream};
use std::io::{Write, Read};

use crate::protocol::{Request, Response, RequestType, ResponseType};

struct CacheItem {
    descritor_arquivo: u32,
    start: u64,
    end: u64,
    data: Vec<u8>,
}

const BUFFER_SIZE: usize = 1024; // 1KB
const MAX_CACHE_SIZE: usize = 1024 * 1024; // 1MB

pub struct  Client {
    cache: LinkedList<CacheItem>,
    server_address: SocketAddr,
}

impl Client {
    pub fn new(server_address: SocketAddr) -> Self {
        Self {server_address, cache: LinkedList::new()}
    }

    pub fn get_server_address(&self) -> SocketAddr {
        self.server_address
    }
    
    /// Envia o buffer para o endereço do servidor
    /// Cria uma stream TCP
    pub fn send(&self, buffer: Vec<u8>, src: SocketAddr) {
        let mut stream = TcpStream::connect(src).unwrap();
        stream.write(&buffer).unwrap();
    }
    
    /// Abre o arquivo no servidor remoto
    /// Retorna 0 se sucesso, -1 se erro
    pub fn abre(&self, descritor_arquivo: i32, nome_arquivo: String) -> i32 {
        // cria a requisição
        let request = Request {
            request_type: RequestType::Abre,
            descritor_arquivo: descritor_arquivo as u32,
            posicao: 0,
            size: 0,
            data: nome_arquivo,            
        };
        let buffer = Request::serialize(request);
        // envia a requisição para o servidor
        self.send(buffer, self.server_address);
        // aguarda a resposta do servidor
        let stream = TcpStream::connect(self.server_address).unwrap();
        let mut buffer: Vec<u8> = vec![0; BUFFER_SIZE];
        stream.take(BUFFER_SIZE as u64).read(&mut buffer).unwrap();
        // desserializa a resposta
        let response = Response::desserialize(&buffer);
        // retorna o código de erro
        response.response_type as i32  
    }
    
    /// Lê do arquivo no servidor remoto
    /// Retorna o número de bytes lidos, -1 se erro
    pub fn le(&mut self, descritor_arquivo: i32, posicao: u64, buffer: &mut Vec<u8>, tamanho: usize) -> i32 {
        // Verifica se o dado está na cache
        for item in self.cache.iter() {
            if item.descritor_arquivo == descritor_arquivo as u32 && item.start <= posicao && item.end >= posicao + tamanho as u64 {
                // copia o dado para o buffer
                let start = (posicao - item.start) as usize;
                let end = start + tamanho;
                buffer.extend(&item.data[start..end]);
                // retorna o número de bytes lidos
                return tamanho as i32;
            }
        }
        // não encontrou na cache
        // cria a requisição
        let request = Request {
            request_type: RequestType::Le,
            descritor_arquivo: descritor_arquivo as u32,
            posicao,
            size: tamanho as u32,
            data: String::new(),            
        };
        let buffer = Request::serialize(request);
        // envia a requisição para o servidor
        self.send(buffer, self.server_address);
        // aguarda a resposta do servidor
        let stream = TcpStream::connect(self.server_address).unwrap();
        let mut buffer: Vec<u8> = vec![0; tamanho + 1];
        stream.take((tamanho + 1) as u64).read(&mut buffer).unwrap();
        // desserializa a resposta
        let response = Response::desserialize(&buffer);
        match response.response_type {
            ResponseType::Ok => {
                // adiciona o dado na cache
                let cache_item = CacheItem {
                    descritor_arquivo: descritor_arquivo as u32,
                    start: posicao,
                    end: posicao + tamanho as u64,
                    data: buffer[1..].to_vec(),
                };
                self.cache.push_back(cache_item);
                // verifica se a cache está maior que o tamanho máximo
                while self.cache.len() > MAX_CACHE_SIZE {
                    self.cache.pop_front();
                }
                // retorna o número de bytes lidos
                return tamanho as i32;
            },
            ResponseType::AtualizaCache => {
                // invalida o dado na cache
                let _ = self.cache.extract_if(|item| item.descritor_arquivo == descritor_arquivo as u32 && item.start >= posicao && item.end <= posicao + tamanho as u64);
                return -1;
            },
            ResponseType::Erro => {
                return -1;
            },
        }
    }
    
    /// Escreve no arquivo no servidor remoto
    /// Retorna 0 se sucesso, -1 se erro
    pub fn escreve(&self, descritor_arquivo: i32, posicao: u64, buffer: &mut Vec<u8>, tamanho: usize) -> i32 {
        // cria a requisição
        let request = Request {
            request_type: RequestType::Escreve,
            descritor_arquivo: descritor_arquivo as u32,
            posicao,
            size: tamanho as u32,
            data: String::from_utf8(buffer[..tamanho].to_vec()).unwrap(),            
        };
        let buffer = Request::serialize(request);
        // envia a requisição para o servidor
        self.send(buffer, self.server_address);
        // aguarda a resposta do servidor
        let stream = TcpStream::connect(self.server_address).unwrap();
        let mut buffer: Vec<u8> = vec![0; BUFFER_SIZE];
        stream.take(tamanho as u64).read(&mut buffer).unwrap();
        // desserializa a resposta
        let response = Response::desserialize(&buffer);
        // retorna o código de erro
        response.response_type as i32
    }

    /// Fecha o arquivo no servidor remoto
    /// Retorna 0 se sucesso, -1 se erro
    pub fn fecha(&mut self, descritor_arquivo: u32) -> i32 {
        let request = Request {
            request_type: RequestType::Fecha,
            descritor_arquivo: descritor_arquivo as u32,
            posicao: 0,
            size: 0,
            data: String::new(),
        };
        let buffer = Request::serialize(request);
        // envia a requisição para o servidor
        self.send(buffer, self.server_address);
        // aguarda a resposta do servidor
        let stream = TcpStream::connect(self.server_address).unwrap();
        let mut buffer: Vec<u8> = vec![0; BUFFER_SIZE];
        stream.take(BUFFER_SIZE as u64).read(&mut buffer).unwrap();
        // desserializa a resposta
        let response = Response::desserialize(&buffer);
        // retorna o código de erro
        response.response_type as i32  
    }
}
