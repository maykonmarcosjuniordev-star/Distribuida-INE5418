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
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::io::{Error, Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use indexmap::IndexMap;
use socket2::{Socket, Domain, Type, Protocol};

const MAX_CACHE_SIZE: usize = 1024 * 1024;
// 1MB
use crate::protocol::{Request, Response, RequestType, ResponseType, BUFFER_SIZE};

#[derive(Clone)]
struct CacheItem {
    descritor_arquivo: i32,
    start: u64,
    end: u64,
    data: Vec<u8>,
}

type Cache = IndexMap<i32, CacheItem>;
pub struct Client {
    cache: Arc<Mutex<Cache>>,
    server_address: SocketAddr,
    client_address: SocketAddr,
    client_id: u32,
}

const DEBUG : bool = true;

impl Client {
    pub fn new(server_address: SocketAddr, client_address: SocketAddr, client_id: u32) -> Self {
        let cache = Arc::new(Mutex::new(IndexMap::new()));

        let cache_clone = Arc::clone(&cache);

        thread::spawn(move || {
            Self::verify_cache_thread(client_address, cache_clone);
        });

        Self {server_address, client_address, cache, client_id}
    }

    pub fn get_server_address(&self) -> SocketAddr {
        self.server_address
    }

    pub fn get_client_address(&self) -> SocketAddr {
        self.client_address
    }

    fn invalidate_cache_if_matches(descriptor: i32, pos: u64, end: u64, c: &mut Cache) {
        c.retain(|_key, item| {
            // 1. Check if the item's file descriptor matches the one being invalidated.
            let is_target_file = item.descritor_arquivo == descriptor;

            // 2. Check for range overlap with the invalidated region.
            //    (The cached item's start is before the written end AND the cached item's end is after the written start)
            let has_overlap = item.start <= end && item.end >= pos;

            // 3. RETENTION LOGIC: 
            //    The closure must return TRUE to KEEP the item.
            //    We KEEP the item ONLY if it is NOT the target file OR if it does NOT overlap.
            //    We REMOVE (return false) if it IS the target file AND it HAS overlap.
            
            !(is_target_file && has_overlap)
        });
    }

    fn verify_cache_thread(client_address: SocketAddr, cache: Arc<Mutex<Cache>>) {
        let listener = match TcpListener::bind(client_address) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Failed to start cache listener on {}: {}", client_address, e);
                return; // Thread exits if listener fails to bind
            }
        };

        // Loop indefinitely, accepting server connections
        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to accept connection: {}", e);
                    continue; // Continue listening for the next connection
                }
            };

            if DEBUG {
                println!("Cache invalidation connection established from {}", stream.peer_addr().unwrap());
            }
            
            let mut temp_buffer: Vec<u8> = vec![0; BUFFER_SIZE];
            // Read the message from the stream
            let bytes_read = stream.read(&mut temp_buffer).unwrap_or(0);
            
            // CRITICAL: Resize the buffer to the actual bytes read
            temp_buffer.resize(bytes_read, 0); 

            // Deserialize and process the invalidation message
            let response = Response::desserialize(&temp_buffer);

            if response.response_type == ResponseType::AtualizaCache {
                // Lock the shared cache before modifying it
                if let Ok(mut c) = cache.lock() {
                    // Perform the invalidation logic on 'c'
                    // This is thread-safe because the Mutex prevents other threads
                    // from accessing 'c' until this block finishes.
                    let (descriptor, pos, size) = Response::parse_atualiza_cache(response);
                    let end = pos + size as u64;
                    
                    Self::invalidate_cache_if_matches(descriptor, pos, end, &mut c);
                    
                    println!("Cache for file {} invalidated by server message.", descriptor);
                } // Mutex lock automatically released here
            }
        }
    }

    

    fn verifica_cache(&self, descritor_arquivo: i32, posicao: u64, buffer: &mut Vec<u8>, tamanho: usize) -> i32 {
        if let Ok(mut c) = self.cache.lock() { 
            let mut item_data: Option<(CacheItem, usize)> = None;
            let end_requested = posicao + tamanho as u64;

            // --- STEP 1: READ and GET INDEX (Creates the problematic mutable borrow 'item') ---
            if let Some((index, _key, item)) = (*c).get_full_mut(&descritor_arquivo) {
                
                if item.start <= posicao && item.end >= end_requested {
                    println!("Cache hit for file descriptor {} at position {}", descritor_arquivo, posicao);
                    
                    if DEBUG {
                        println!("Cache item details: start={}, end={}, data_len={}", item.start, item.end, item.data.len());
                        println!("Requested details: posicao={}, tamanho={}, end_requested={}", posicao, tamanho, end_requested);
                    }

                    // 1. CLONE/EXTRACT all necessary information *while* the borrow is valid.
                    let offset_start = (posicao - item.start) as usize;
                    let offset_end = offset_start + tamanho;

                    if item.data.len() >= offset_end {
                        // Store the necessary data and the index for later use.
                        // We use clone() on the item's data to break the link to the mutable borrow.
                        item_data = Some((item.clone(), index)); 
                    } else {
                        eprintln!("Cache corruption: Data size mismatch. Falling through to server read.");
                    }
                }
            } else {
                return -1; // File descriptor not found in cache
            }
            // The mutable borrow created by get_full_mut() ENDS HERE. 
            // Rust is now happy to use 'c' again.

            // --- STEP 2: REPOSITION AND COPY (Uses the extracted info) ---
            if let Some((item_to_copy, index)) = item_data {
                
                // Repositioning must happen BEFORE copying data out, to keep the cache consistent
                // c is mutably borrowed here, but NO conflicting borrow exists!
                let new_index = c.len() - 1; 
                (*c).move_index(index, new_index); 

                // Copy data to the output buffer
                let offset_start = (posicao - item_to_copy.start) as usize;
                let offset_end = offset_start + tamanho;

                buffer.extend(&item_to_copy.data[offset_start..offset_end]);
                
                return tamanho as i32;
            } else {
                return -1;
            }
        }
        return -1;
    }

    fn cache_update_write(&self, new_item: CacheItem) {
        if let Ok(mut c) = self.cache.lock() {
            // 1. Invalidation/Cleanup (Required before insertion)
            // Remove any overlapping or contained cache items for this file/region.
            // This prevents old, partial data from conflicting with the new, authoritative data.
            let end = new_item.end;
            let pos = new_item.start;

            Self::invalidate_cache_if_matches(new_item.descritor_arquivo, pos, end, &mut c);
            self.cache_write(new_item, &mut c);
        }
    }

    fn cache_write(&self, new_item: CacheItem, c: &mut Cache) {
        // 2. Insert the new, authoritative data
        // If the descriptor key already exists, IndexMap updates it and moves it to the MRU end.
        c.insert(new_item.descritor_arquivo, new_item);

        // 3. Enforce the size limit (Eviction)
        while c.len() > MAX_CACHE_SIZE {
            if let Some((_key, removed_item)) = (*c).pop() {
                println!("Evicted item for file descriptor {}", removed_item.descritor_arquivo);
            }
        }
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
        // socket.bind(&self.client_address.into())?; // REMOVIDO pois já esta bindado pela thread de verify cache
        // Conecta ao servidor
        socket.connect(&self.server_address.into())?;
        // Converte para TcpStream
        let mut stream: TcpStream = socket.into();
        // Send request
        stream.write(&buffer)?;
        stream.flush()?;
        // Aguarda a resposta
        let mut buffer: Vec<u8> = vec![0; BUFFER_SIZE];
        let bytes_received = stream.read(&mut buffer)?;
        if bytes_received > 0 {
            buffer.resize(bytes_received, 0);
        }
        // desserializa a resposta
        let response = Response::desserialize(&buffer);
        return Ok(response);
    }
    
    /// Abre o arquivo no servidor remoto
    /// Retorna 0 se sucesso, -1 se erro
    pub fn abre(&self, descritor_arquivo: i32, nome_arquivo: String) -> i32 {
        // cria a requisição
        let data = nome_arquivo.into_bytes();
        let client_id = self.client_id;
        let request = Request {
            request_type: RequestType::Abre,
            descritor_arquivo: descritor_arquivo,
            posicao: 0,
            size: data.len() as u32,
            client_id,
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
        
        let cache_result = self.verifica_cache(descritor_arquivo, posicao, buffer, tamanho);
        if cache_result != -1 {
            return cache_result; // cache hit
        }

        // não encontrou na cache
        // cria a requisição
        let client_id = self.client_id;
        let request = Request {
            request_type: RequestType::Le,
            descritor_arquivo,
            posicao,
            size: tamanho as u32,
            client_id,
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
                let bytes_received = response.data.len();
                // copia o dado para o buffer
                if tamanho > bytes_received {
                    println!("Warning: Expected to read {} bytes but received only {}", tamanho, bytes_received);
                    buffer.extend(&response.data);
                } else {
                    buffer.extend(&response.data[..tamanho]);
                }
                // adiciona o dado na cache
                let cache_item = CacheItem {
                    descritor_arquivo: descritor_arquivo,
                    start: posicao,
                    end: posicao + tamanho as u64,
                    data: buffer.to_vec(),
                };
                if let Ok(mut c) = self.cache.lock() {
                    self.cache_write(cache_item, &mut c);
                }
                // retorna o número de bytes lidos
                return bytes_received as i32;
            },
            _ => {
                return -1;
            },
        }
    }
    
    /// Escreve no arquivo no servidor remoto
    /// Retorna 0 se sucesso, -1 se erro
    pub fn escreve(&self, descritor_arquivo: i32, posicao: u64,
                    buffer: &mut Vec<u8>, tamanho: usize) -> i32 {
        // cria a requisição
        let client_id = self.client_id;
        let request = Request {
            request_type: RequestType::Escreve,
            descritor_arquivo: descritor_arquivo,
            posicao,
            size: tamanho as u32,
            client_id,
            data: buffer[..tamanho].to_vec(),
        };
        // envia a requisição para o servidor e aguarda a resposta
        let response = match self.send(request) {
            Ok(resp) => {
                println!("Write request successful, updating cache.");
                // Atualiza a cache local (invalidação)
                let cache_item = CacheItem {
                    descritor_arquivo: descritor_arquivo,
                    start: posicao,
                    end: posicao + tamanho as u64,
                    data: buffer[..tamanho].to_vec(),
                };
                self.cache_update_write(cache_item);
                resp
            },
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
        let client_id = self.client_id;
        let request = Request {
            request_type: RequestType::Fecha,
            descritor_arquivo: descritor_arquivo,
            posicao: 0,
            size: 0,
            client_id,
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
