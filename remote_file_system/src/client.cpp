#include <sys/socket.h>

using namespace std;


const int MAX_CACHE_SIZE = 1024 * 1024;
const int BUFFER_SIZE = 1024;

struct {
    int descritor_arquivo; // i32
    long long start; // u64
    long long end; // u64
    vector<char> data; // u8
} CacheItem;

enum RequestType {
    ABRE,
    LE,
    ESCREVE,
    FECHA
};

struct Request {
    RequestType request_type;
    int descritor_arquivo;
    int posicao;
    int size;
    char data[];
};



enum ResponseType {
    OK,
    CACHE_UPDATE,
    ERRO,
};

struct Response {
    ResponseType response_type;
    char data[];
};


class Client {
public:
    vector<CacheItem> cache;
    struct sockaddr_in server_address;
    int socket;

    sockaddr_in get_server_address(self) {
        return server_address;
    }

    int send(char buffer[]) {
        int result;
        int len = sizeof(server_address);
        result = connect(socket, (struct sockaddr *)&server_address, len);
        if (result == -1) {
            perror("AAAAAAAAAAAAAAAAAAAAAAAAA"); // TODO
            return -1;
        }
        len = buffer.len();

        write(socket, &buffer, len);
        read(socket, &buffer, len);

        close(socket);
    }

    void setup() {
        socket = socket(AF_INET, SOCK_STREAM, 0);
        server_address.sun_family = AF_INET;
        server_address.sin_port = htons(8080); // E a porta?
        server_address.sin_addr.s_addr = inet_addr("127.0.0.1"); // Como fazer o IP do servidor?
    }

    int abre(int descritor_arquivo, string nome_arquivo) {
        Request rqst;
        rqst.request_type = RequestType::ABRE;
        rqst.descritor_arquivo = descritor_arquivo;
        rqst.posicao = 0;
        rqst.size = 0;
        rqst.data = nome_arquivo;
        
        
        char buffer[BUFFER_SIZE];
        buffer = serialize(rqst);
        send(buffer);

        Response rsp = desserialize(buffer);
        return rsp.response_type;
    }

    int fecha(int descritor_arquivo) {
        Request rqst;
        rqst.request_type = RequestType::FECHA;
        rqst.descritor_arquivo = descritor_arquivo;
        rqst.posicao = 0;
        rqst.size = 0;
        rqst.data = [];
        
        
        char buffer[BUFFER_SIZE];
        buffer = serialize(rqst);
        send(buffer);
        Response rsp = desserialize(buffer);
        
        return rsp.response_type;
    }

    int escreve(int descritor_arquivo, int posicao, char[] buffer, int tamanho) {
        Request rqst;
        rqst.request_type = RequestType::ESCREVE;
        rqst.descritor_arquivo = descritor_arquivo;
        rqst.posicao = posicao;
        rqst.size = tamanho;
        rqst.data = buffer;
        
        
        char buffer[BUFFER_SIZE];
        buffer = serialize(rqst);
        send(buffer);
        Response rsp = desserialize(buffer);
        
        return rsp.response_type;
    }

    void verify_cache() {
        
    }


    void serialize() {} // TODO
    void desserialize() {} // TODO
}


// impl Client {
//     fn verify_cache(&mut self) {
//         // checa se a cache não está inválida
//         let listener = TcpListener::bind(server_address).unwrap();
//         for stream in listener.incoming() {
//             let mut stream = stream.unwrap();
//             let mut temp_buffer: Vec<u8> = vec![0; BUFFER_SIZE];
//             stream.read(&mut temp_buffer).unwrap();
//             let response = Response::desserialize(&temp_buffer);
//             if response.response_type == ResponseType::AtualizaCache {
//                 // invalida o dado na cache
//                 let data = response.data;
//                 let descriptor = i32::from_be_bytes(data[0..4].try_into().unwrap());
//                 let pos = u64::from_be_bytes(data[4..12].try_into().unwrap());
//                 let ssize = u64::from_be_bytes(data[12..20].try_into().unwrap());
//                 let _ = cache.extract_if(|item|
//                                         item.descritor_arquivo == descriptor
//                                         && item.start >= pos
//                                         && item.end <= pos + ssize);
//             }
//         }
//     }
    
//     pub fn le(&mut self, descritor_arquivo: i32, posicao: u64, buffer: &mut Vec<u8>, tamanho: usize) -> i32 {
//         // Verifica se o dado está na cache
//         let mut item_opt: Option<CacheItem> = None;
//         for item in &cache {
//             if item.descritor_arquivo == descritor_arquivo && item.start <= posicao && item.end >= posicao + tamanho as u64 {
//                 item_opt = Some(item.clone());
//                 break;
//             }
//         }
//         if !item_opt.is_none() {
//             // checa se a cache não está inválida
//             verify_cache();
//             // se ainda está na cache, lê dela
//             let start = (posicao - item_opt.as_ref().unwrap().start) as usize;
//             let end = start + tamanho;
//             // copia o dado para o buffer
//             buffer.extend(&item_opt.as_ref().unwrap().data[start..end]);
//             // retorna o número de bytes lidos
//             return tamanho as i32;
//         }
//         // não encontrou na cache
//         // cria a requisição
//         let request = Request {
//             request_type: RequestType::Le,
//             descritor_arquivo,
//             posicao,
//             size: tamanho as u32,
//             data: vec![],
//         };
//         let buffer = Request::serialize(request);
//         // envia a requisição para o servidor
//         Self::send(buffer, server_address);
//         // aguarda a resposta do servidor
//         let stream = TcpStream::connect(server_address).unwrap();
//         let mut buffer: Vec<u8> = vec![0; tamanho + 1];
//         stream.take((tamanho + 1) as u64).read(&mut buffer).unwrap();
//         // desserializa a resposta
//         let response = Response::desserialize(&buffer);
//         match response.response_type {
//             ResponseType::Ok => {
//                 // adiciona o dado na cache
//                 let cache_item = CacheItem {
//                     descritor_arquivo: descritor_arquivo,
//                     start: posicao,
//                     end: posicao + tamanho as u64,
//                     data: buffer[1..].to_vec(),
//                 };
//                 cache.push_back(cache_item);
//                 // verifica se a cache está maior que o tamanho máximo
//                 while cache.len() > MAX_CACHE_SIZE {
//                     cache.pop_front();
//                 }
//                 // retorna o número de bytes lidos
//                 return tamanho as i32;
//             },
//             ResponseType::AtualizaCache => {
//                 // invalida o dado na cache
//                 let _ = cache.extract_if(|item| item.descritor_arquivo == descritor_arquivo && item.start >= posicao && item.end <= posicao + tamanho as u64);
//                 return -1;
//             },
//             ResponseType::Erro => {
//                 return -1;
//             },
//         }
//     }
  
// }


// use std::net::{SocketAddr, TcpStream, TcpListener};
// use std::io::{Write, Read};

// use crate::protocol::{Request, Response, RequestType, ResponseType, BUFFER_SIZE};


// impl Request {
//     /// Desserializa o buffer recebido do cliente
//     pub fn desserialize(buffer: &Vec<u8>) -> Request {
//         // Expected layout:
//         // [0]                 -> request_type (1 byte)
//         // [1..5]              -> descritor_arquivo (4 bytes, big-endian)
//         // [5..9]              -> posicao (4 bytes, big-endian)
//         // [9..13]             -> size (4 bytes, big-endian)
//         // [13..]              -> data (remaining bytes)
//         Request {
//             request_type: match buffer[0] {
//                 0 => RequestType::Abre,
//                 1 => RequestType::Le,
//                 2 => RequestType::Escreve,
//                 3 => RequestType::Fecha,
//                 _ => panic!("Invalid request type"),
//             },
//             descritor_arquivo: i32::from_be_bytes(buffer[1..5].try_into().unwrap()),
//             posicao: u64::from_be_bytes(buffer[5..13].try_into().unwrap()),
//             size: u32::from_be_bytes(buffer[13..17].try_into().unwrap()),
//             data: buffer[17..].to_vec(),
//         }
//     }
//     pub fn serialize(response: Request) -> Vec<u8> {
//         let mut buffer: Vec<u8> = vec![];
//         buffer.push(response.request_type as u8);
//         buffer.extend(&response.descritor_arquivo.to_be_bytes());
//         buffer.extend(&response.posicao.to_be_bytes());
//         buffer.extend(&response.size.to_be_bytes());
//         buffer.extend(response.data);
//         buffer
//     }
// }

// impl Response {
//     pub fn serialize(response: Response) -> Vec<u8> {
//         let mut buffer: Vec<u8> = vec![];
//         buffer.push(response.response_type as u8);
//         buffer.extend(response.data);
//         buffer
//     }
//     pub fn desserialize(buffer: &Vec<u8>) -> Response {
//         Response {
//             response_type: match buffer[0] {
//                 0 => ResponseType::Ok,
//                 1 => ResponseType::AtualizaCache,
//                 2 => ResponseType::Erro,
//                 _ => panic!("Invalid response type"),
//             },
//             data: buffer[1..].to_vec(),
//         }
//     }
// }