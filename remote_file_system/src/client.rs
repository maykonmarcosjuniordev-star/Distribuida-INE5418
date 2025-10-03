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
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::io::{Error, Read, Write};
use std::sync::{Arc, Mutex};
use socket2::{Socket, Domain, Type, Protocol};

const MAX_CACHE_SIZE: usize = 1024 * 1024; // 1MB
use crate::protocol::{Request, Response, RequestType, ResponseType, BUFFER_SIZE};

#[derive(Clone)]
struct CacheItem {
    descritor_arquivo: i32,
    start: u64,
    end: u64,
    data: Vec<u8>,
}

pub struct  Client {
    cache: Arc<Mutex<LinkedList<CacheItem>>>,
    server_address: SocketAddr,
    client_address: SocketAddr,
}

impl Client {
    pub fn new(server_address: SocketAddr, client_address: SocketAddr) -> Self {
        let cache = Arc::new(Mutex::new(LinkedList::new()));
        Self {server_address, client_address, cache}
    }

    pub fn get_server_address(&self) -> SocketAddr {
        self.server_address
    }

    pub fn get_client_address(&self) -> SocketAddr {
        self.client_address
    }
    
    /// Envia o buffer para o endereço do servidor
    /// E recebe a resposta do servidor
    /// Creando uma stream TCP
    fn send(&self, request: Request) -> Result<Response, Error> {
        // Serializa a requisição
        let buffer = Request::serialize(request);
        // Cria o socket
        let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
        // Define opções do socket
        socket.set_reuse_address(true)?;
        // Liga o socket ao endereço do cliente
        socket.bind(&self.client_address.into())?;
        // Conecta ao servidor
        socket.connect(&self.server_address.into())?;
        // Converte para TcpStream
        let mut stream: TcpStream = socket.into();
        // Send request
        stream.write(&buffer)?;
        stream.flush()?;
        // Aguarda a resposta
        let mut buffer: Vec<u8> = vec![0; BUFFER_SIZE];
        stream.read(&mut buffer)?;
        // desserializa a resposta
        let response = Response::desserialize(&buffer);
        return Ok(response);
    }
    
    /// checa se a cache não está inválida
    fn verify_cache(&self) {
        let listener = match TcpListener::bind(self.client_address) {
            Ok(l) => {
                println!("Cache listener started on {}", self.client_address);
                l
            },
            Err(e) => {
                println!("Failed to start cache listener on {}: {}", self.client_address, e);
                return;
            }
        };
        listener.set_nonblocking(true).expect("Cannot set non-blocking");
        // aguarda por mensagens de invalidação
        for stream in listener.incoming() {
            let mut stream = stream.expect("Failed to accept connection");
            let mut temp_buffer: Vec<u8> = vec![0; BUFFER_SIZE];
            stream.read(&mut temp_buffer).expect("Failed to read from stream");
            let response = Response::desserialize(&temp_buffer);
            if response.response_type == ResponseType::AtualizaCache {
                // invalida o dado na cache
                let (descriptor, pos, size) = Response::parse_atualiza_cache(response);
                let end = pos + size as u64;
                if let Ok(mut c) = self.cache.lock() {
                    println!("Current cache size: {}", c.len());
                    let _ = c.extract_if(|item|
                        item.descritor_arquivo == descriptor
                        && (item.start <= end
                            && item.end >= pos)
                        );
                }
                println!("Cache invalidation for file descriptor {} at position {}", descriptor, pos);
            }
        }
    }
    
    /// Abre o arquivo no servidor remoto
    /// Retorna 0 se sucesso, -1 se erro
    pub fn abre(&self, descritor_arquivo: i32, nome_arquivo: String) -> i32 {
        // cria a requisição
        let data = nome_arquivo.into_bytes();
        let request = Request {
            request_type: RequestType::Abre,
            descritor_arquivo: descritor_arquivo,
            posicao: 0,
            size: data.len() as u32,
            data
        };
        // envia a requisição para o servidor e aguarda a resposta
        let response = match self.send(request) {
            Ok(resp) => resp,
            Err(e) => {
                println!("Erro ao receber resposta do servidor na função abre: {}", e);
                return -1;
            }
        };
        // retorna o código de erro
        response.response_type as i32  
    }

    /// Lê do arquivo no servidor remoto
    /// Retorna o número de bytes lidos, -1 se erro
    pub fn le(&self, descritor_arquivo: i32, posicao: u64, buffer: &mut Vec<u8>, tamanho: usize) -> i32 {
        // Verifica se o dado está na cache
        if let Ok(c) = self.cache.lock() {
            for item in &*c {
                if item.descritor_arquivo == descritor_arquivo && item.start <= posicao && item.end >= posicao + tamanho as u64 {
                    println!("Cache hit for file descriptor {} at position {}", descritor_arquivo, posicao);
                    self.verify_cache();
                    // se ainda está na cache, lê dela
                    let start = (posicao - item.start) as usize;
                    let end = start + tamanho;
                    // copia o dado para o buffer
                    buffer.extend(&item.data[start..end]);
                    // retorna o número de bytes lidos
                    return tamanho as i32;
                }
            }
        }
        // não encontrou na cache
        // cria a requisição
        let request = Request {
            request_type: RequestType::Le,
            descritor_arquivo,
            posicao,
            size: tamanho as u32,
            data: vec![],
        };
        // envia a requisição para o servidor e aguarda a resposta
        let response = match self.send(request) {
            Ok(resp) => resp,
            Err(e) => {
                println!("Erro ao receber resposta do servidor na função le: {}", e);
                return -1;
            }
        };
        // processa a resposta, atualiza a cache se necessário
        match &response.response_type {
            ResponseType::Ok => {
                // copia o dado para o buffer
                buffer.extend(&response.data[..tamanho]);
                // adiciona o dado na cache
                let cache_item = CacheItem {
                    descritor_arquivo: descritor_arquivo,
                    start: posicao,
                    end: posicao + tamanho as u64,
                    data: buffer[..tamanho].to_vec(),
                };
                if let Ok(mut c) = self.cache.lock() {
                    c.push_back(cache_item);
                    while c.len() > MAX_CACHE_SIZE {
                        c.pop_front();                        
                    }
                }
                // retorna o número de bytes lidos
                return tamanho as i32;
            },
            ResponseType::AtualizaCache => {
                // invalida o dado na cache
                println!("Cache invalidation for file descriptor {} at position {}", descritor_arquivo, posicao);
                let _ = self.cache
                            .lock()
                            .expect("Failed to lock cache")
                            .extract_if(|item| 
                            item.descritor_arquivo == descritor_arquivo 
                            && item.start >= posicao
                            && item.end <= posicao + tamanho as u64);
                return -1;
            },
            ResponseType::Erro => {
                return -1;
            },
        }
    }
    
    /// Escreve no arquivo no servidor remoto
    /// Retorna 0 se sucesso, -1 se erro
    pub fn escreve(&self, descritor_arquivo: i32, posicao: u64,
                    buffer: &mut Vec<u8>, tamanho: usize) -> i32 {
        // cria a requisição
        let request = Request {
            request_type: RequestType::Escreve,
            descritor_arquivo: descritor_arquivo,
            posicao,
            size: tamanho as u32,
            data: buffer[..tamanho].to_vec(),
        };
        // envia a requisição para o servidor e aguarda a resposta
        let response = match self.send(request) {
            Ok(resp) => resp,
            Err(e) => {
                println!("Erro ao receber resposta do servidor na função escreve: {}", e);
                return -1;
            }
        };
        // retorna o código de erro
        response.response_type as i32
    }

    /// Fecha o arquivo no servidor remoto
    /// Retorna 0 se sucesso, -1 se erro
    pub fn fecha(&self, descritor_arquivo: i32) -> i32 {
        let request = Request {
            request_type: RequestType::Fecha,
            descritor_arquivo: descritor_arquivo,
            posicao: 0,
            size: 0,
            data: vec![],
        };
        // envia a requisição para o servidor e aguarda a resposta
        let response = match self.send(request) {
            Ok(resp) => resp,
            Err(e) => {
                println!("Erro ao receber resposta do servidor na função fecha: {}", e);
                return -1;
            }
        };
        // retorna o código de erro
        response.response_type as i32  
    }
}
