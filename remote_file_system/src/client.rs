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
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::io::{Error, Read, Write};
use std::sync::Mutex;
use socket2::{Socket, Domain, Type, Protocol};

const MAX_CACHE_SIZE: usize = 1024 * 1024; // 1MB
use crate::protocol::{Request, Response, ResponseType, RequestFactory, StandardRequestFactory, BUFFER_SIZE};

#[derive(Clone)]
struct CacheItem {
    descritor_arquivo: i32,
    start: u64,
    end: u64,
    data: Vec<u8>,
}

pub struct  Client {
    cache: Mutex<LinkedList<CacheItem>>,
    server_address: SocketAddr,
    client_address: SocketAddr,
    request_factory: StandardRequestFactory,
    warning_socket: UdpSocket,
    client_id: u32,
}
macro_rules! client_log {
    // Rule for the case where arguments are provided (like println!("Error: {}", e))
    ($client:expr, $fmt:literal $(, $args:expr)*) => {
        // Use eprintln! to combine the dynamic prefix and the formatted message
        eprintln!(
            // The format string starts with the prefix placeholder, followed by the message
            concat!("[Client {}]: ", $fmt), 
            // The first argument is the client ID, followed by all arguments for the message
            $client.client_id, 
            $($args),*
        )
    };
    // Rule for the case where NO arguments are provided (like println!("Status ok"))
    ($client:expr, $fmt:literal) => {
        eprintln!(concat!("[Client {}]: ", $fmt), $client.client_id)
    };
}

impl Client {
    pub fn new(server_address: SocketAddr, client_address: SocketAddr, client_id: u32) -> Self {
        let cache = Mutex::new(LinkedList::new());
        let request_factory = StandardRequestFactory;
        let warning_socket = UdpSocket::bind(client_address)
            .expect("Failed to bind UDP socket for warnings");
        warning_socket
            .set_nonblocking(true)
            .expect("Failed to set non-blocking mode");
        Self {server_address, client_address, cache, request_factory, warning_socket, client_id}
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
        // Serializa a requisição
        let buffer = Request::serialize(&request);
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

    fn invalidate_cache(&self, response: &Response) {
        // invalida o dado na cache
        let (descriptor, pos, size) = Response::parse_atualiza_cache(response);
        client_log!(self, "Cache invalidation for file descriptor {} at position {}", descriptor, pos);
        let end = pos + size as u64;
        if let Ok(mut c) = self.cache.lock() {
            // client_log!(self, "Current cache size: {}", c.len());
            client_log!(self, "Invalidating cache for file descriptor {} from position {} to {} and size {}", descriptor, pos, end, size);
            // printa itens da cache
            // for item in &*c {
            //     client_log!(self, "Cache item - file descriptor {} from position {} to {}", item.descritor_arquivo, item.start, item.end);
            // }
            c.extract_if(|item|
                item.descritor_arquivo == descriptor
                && item.start <= end 
                && item.end >= pos
                ).for_each(drop);
            
            // client_log!(self, "New cache size: {}", c.len());
        }
    }

    /// checa se a cache não está inválida
    /// Usando um listener UDP
    /// Se receber uma mensagem de invalidação, remove o item da cache
    /// Retorna -1 se erro, 0 se não há mensagens, n se há n mensagens de invalidação de cache
    fn verify_cache(&self) -> i32 {
        client_log!(self, "Verifying cache for client at address {}", self.client_address);
        let mut buf = [0; BUFFER_SIZE];
        let mut output = 0;
        while let Ok((amt, src)) = self.warning_socket.recv_from(&mut buf) {
            client_log!(self, "Received warning on cache listener, {} bytes from {}", amt, src);
            let response = Response::desserialize(&buf.to_vec());
            match response.response_type {
                ResponseType::AtualizaCache => {
                        output += 1;
                        self.invalidate_cache(&response)
                    },
                    ResponseType::Ok => {
                        output = -1;
                        client_log!(self, " --- Received OK message on cache listener")
                    },
                    ResponseType::Erro => {
                        output = -1;
                        client_log!(self, " --- Received Error message on cache listener")
                    },
                }
            }
        return output;
    }
    
    /// Abre o arquivo no servidor remoto
    /// Retorna 0 se sucesso, -1 se erro
    pub fn abre(&self, descritor_arquivo: i32, nome_arquivo: String) -> i32 {
        // cria a requisição
        client_log!(self, "Opening file {} with descriptor {}", nome_arquivo, descritor_arquivo);
        let request = self.request_factory.create_open_request(descritor_arquivo, nome_arquivo);
        // envia a requisição para o servidor e aguarda a resposta
        let response = match self.send(request) {
            Ok(resp) => {
                client_log!(self, "Successfully opened file with descriptor {}", descritor_arquivo);
                resp
            },
            Err(e) => {
                client_log!(self, "Erro ao receber resposta do servidor na função abre: {}", e);
                return -1;
            }
        };
        // retorna o código de erro
        response.response_type as i32  
    }

    /// Lê do arquivo no servidor remoto
    /// Retorna o número de bytes lidos, -1 se erro
    pub fn le(&self, descritor_arquivo: i32, posicao: u64, buffer: &mut Vec<u8>, tamanho: usize) -> i32 {
        client_log!(self, "Reading {} bytes from file descriptor {} at position {}", tamanho, descritor_arquivo, posicao);
        // Verifica se o dado está na cache
        self.verify_cache();
        if let Ok(c) = self.cache.lock() {
            for item in &*c {
                if item.descritor_arquivo == descritor_arquivo
                    && item.start <= posicao
                    && item.end >= posicao + tamanho as u64
                {
                    client_log!(self, "Cache hit for file descriptor {} at position {}", descritor_arquivo, posicao);
                    // se ainda está na cache, lê dela
                    let start = (posicao - item.start) as usize;
                    let end = start + tamanho;
                    // copia o dado para o buffer
                    buffer.extend(&item.data[start..end]);
                    client_log!(self, "Read message from cache: {:?}", String::from_utf8_lossy(&item.data[start..end]).trim());
                    // retorna o número de bytes lidos
                    return tamanho as i32;
                }
            }
        }
        client_log!(self, "Cache miss for file descriptor {} at position {}", descritor_arquivo, posicao);
        // não encontrou na cache
        // cria a requisição
        let request = self.request_factory.create_read_request(descritor_arquivo, posicao, tamanho);
        // envia a requisição para o servidor e aguarda a resposta
        let response = match self.send(request) {
            Ok(resp) => resp,
            Err(e) => {
                client_log!(self, "Erro ao receber resposta do servidor na função le: {}", e);
                return -1;
            }
        };
        // processa a resposta, atualiza a cache se necessário
        match &response.response_type {
            ResponseType::Ok => {
                // copia o dado para o buffer
                buffer.extend(&response.data[..tamanho]);
                client_log!(self, "Read message from server: {:?}", String::from_utf8_lossy(&response.data[..tamanho]).trim());
                // adiciona o dado na cache
                let cache_item = CacheItem {
                    descritor_arquivo: descritor_arquivo,
                    start: posicao,
                    end: posicao + tamanho as u64,
                    data: buffer[..tamanho].to_vec(),
                };
                if let Ok(mut c) = self.cache.lock() {
                    // adiciona o item na cache
                    c.push_back(cache_item);
                    // Garante que a cache não ultrapasse o tamanho máximo
                    while c.len() > MAX_CACHE_SIZE {
                        c.pop_front();                        
                    }
                }
                // retorna o número de bytes lidos
                return tamanho as i32;
            },
            ResponseType::AtualizaCache => {
                // invalida o dado na cache
                client_log!(self, "Received cache invalidation response from server instead of data");
                self.invalidate_cache(&response);
                return -1;
            },
            ResponseType::Erro => {
                client_log!(self, "Server returned error for read request on file descriptor {}", descritor_arquivo);
                return -1;
            },
        }
    }
    
    /// Escreve no arquivo no servidor remoto
    /// Retorna 0 se sucesso, -1 se erro
    pub fn escreve(&self, descritor_arquivo: i32, posicao: u64,
                    buffer: &Vec<u8>, tamanho: usize) -> i32 {
        client_log!(self, "Writing {} bytes to file descriptor {} at position {}", tamanho, descritor_arquivo, posicao);
        // cria a requisição
        let request = self.request_factory.create_write_request(descritor_arquivo, posicao, buffer, tamanho);
        // envia a requisição para o servidor e aguarda a resposta
        let response = match self.send(request) {
            Ok(resp) => {
                client_log!(self, "Successfully wrote to file descriptor {}", descritor_arquivo);
                resp
            },
            Err(e) => {
                client_log!(self, "Erro ao receber resposta do servidor na função escreve: {}", e);
                return -1;
            }
        };
        // retorna o código de erro
        response.response_type as i32
    }

    /// Fecha o arquivo no servidor remoto
    /// Retorna 0 se sucesso, -1 se erro
    pub fn fecha(&self, descritor_arquivo: i32) -> i32 {
        client_log!(self, "Closing file descriptor {}", descritor_arquivo);
        let request = self.request_factory.create_close_request(descritor_arquivo);
        // envia a requisição para o servidor e aguarda a resposta
        match self.send(request) {
            Ok(resp) => {
                client_log!(self, "Successfully closed file descriptor {}", descritor_arquivo);
                return match resp.response_type {
                    ResponseType::Ok => 0,
                    _ => -1,
                };
            },
            Err(e) => {
                client_log!(self, "Erro ao receber resposta do servidor na função fecha: {}", e);
                return -1;
            }
        };
    }
}
