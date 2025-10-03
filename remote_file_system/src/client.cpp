#include <sys/socket.h>
#include <vector>

using namespace std;


const int MAX_CACHE_SIZE = 1024;
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
    vector<char> data;
};

enum ResponseType {
    OK,
    CACHE_UPDATE,
    ERRO,
};

struct Response {
    ResponseType response_type;
    vector<char> data;
};


class Client {
public:
    vector<CacheItem> cache;
    struct sockaddr_in server_address;
    int socket;

    void setup() {
        socket = socket(AF_INET, SOCK_STREAM, 0);
        server_address.sun_family = AF_INET;
        server_address.sin_port = htons(8080); // E a porta?
        server_address.sin_addr.s_addr = inet_addr("127.0.0.1"); // Como fazer o IP do servidor?
    }

    int sendServer(vector<char> buffer, Request rqst) {
        int result;
        int len = sizeof(server_address);
        result = connect(socket, (struct sockaddr *)&server_address, len);
        if (result == -1) {
            perror("AAAAAAAAAAAAAAAAAAAAAAAAA"); // TODO
            return -1;
        }

        write(socket, &rqst, sizeof(rqst));
        read(socket, &buffer, len);

        close(socket);
    }

    int abre(int descritor_arquivo, string nome_arquivo) {
        Request rqst;
        rqst.request_type = RequestType::ABRE;
        rqst.descritor_arquivo = descritor_arquivo;
        rqst.posicao = 0;
        rqst.size = 0;
        rqst.data = nome_arquivo;
        
        
        char buffer[BUFFER_SIZE];
        sendServer(buffer, rqst);

        Response rsp = desserialize(buffer);
        return rsp.response_type;
    }

    int fecha(int descritor_arquivo) {
        Request rqst;
        rqst.request_type = RequestType::FECHA;
        rqst.descritor_arquivo = descritor_arquivo;
        rqst.posicao = 0;
        rqst.size = 0;
        rqst.data = {}; // TODO check
        
        
        char buffer[BUFFER_SIZE];
        sendServer(buffer, rqst);

        Response rsp = desserialize(buffer);
        return rsp.response_type;
    }

    int escreve(int descritor_arquivo, unsigned long long posicao, vector<char> &buffer, int tamanho) {
        Request rqst;
        rqst.request_type = RequestType::ESCREVE;
        rqst.descritor_arquivo = descritor_arquivo;
        rqst.posicao = posicao;
        rqst.size = tamanho;
        rqst.data = buffer;
        
        
        char buffer[BUFFER_SIZE];
        sendServer(buffer, rqst);
        
        Response rsp = desserialize(buffer);
        return rsp.response_type;
    }

    int le(int descritor_arquivo, unsigned long long posicao, vector<char> &buffer, int tamanho) {
        for (auto item : cache) {
            if (item.descritor_arquivo == descritor_arquivo and item.start <= posicao and posicao+tamanho <= item.end) {
                int cacheStatus = verify_cache();
                if (cacheStatus) {
                    int start = posicao - item.start;
                    int end = start+tamanho;
                    buffer = item.data[start..end];
                    return tamanho;
                } else {
                    // Remove item from cache?
                }
            }
        }

        Request rqst;
        rqst.request_type = RequestType::LE;
        rqst.descritor_arquivo = descritor_arquivo;
        rqst.posicao = posicao;
        rqst.size = tamanho;
        rqst.data = {};

        sendServer(buffer, rqst);

        Response rsp = desserialize(buffer);

        if (rsp.response_type == ResponseType::OK) {
            CacheItem item;
            item.descritor_arquivo = descritor_arquivo;
            item.start = posicao;
            item.end = posicao+tamanho;
            item.data = rsp.data;
            cache.push_back(item);
            if (cache.size() > MAX_CACHE_SIZE) {
                cache.erase(cache.begin()); // TODO: change to better structure, since this is O(N)
            }
            return tamanho;
        } else { // ERROR
            return -1;
        }
    }


    void verify_cache() {
        int result;
        char buffer[BUFFER_SIZE];
        result = connect(socket, (struct sockaddr *)&server_address, len);
        if (result == -1) {
            perror("AAAAAAAAAAAAAAAAAAAAAAAAA"); // TODO
            return -1;
        }


        result = recv(socket, &buffer, len, PEEK);

        if (result) {
            read(socket, &buffer, len);
            Response rsp = desserialize(buffer);
            if (rsp.response_type == CACHE_UPDATE) {
                vector<char> data = rsp.data;
                int descriptor = data[0..4] // TODO how to desserialize
                int posicao = data[4..12] // TODO how to desserialize
                int sizee = data[12..20] // TODO how to desserialize
                for (auto item : cache) {
                    remove if item.descritor_arquivo == descriptor
                                        && item.start >= pos
                                        && item.end <= pos + ssize);
                }
            }
        }
    }



    int serialize(Request &rqst) {
        int32_t byte0 = htonl(rqst.request_type);
        int32_t byte4 = htonl(rqst.descritor_arquivo);
        int32_t byte8 = htonl(rqst.posicao);
        int32_t byte12 = htonl(rqst.size);
        int32_t byte16e = htonl((int32_t)rqst.data.size());
    }
}

    RequestType request_type;
    int descritor_arquivo;
    int posicao;
    int size;
    vector<char> data;


use std::net::{SocketAddr, TcpStream, TcpListener};
use std::io::{Write, Read};

use crate::protocol::{Request, Response, RequestType, ResponseType, BUFFER_SIZE};


impl Request {
    /// Desserializa o buffer recebido do cliente
    pub fn desserialize(buffer: &Vec<u8>) -> Request {
        // Expected layout:
        // [0]                 -> request_type (1 byte)
        // [1..5]              -> descritor_arquivo (4 bytes, big-endian)
        // [5..9]              -> posicao (4 bytes, big-endian)
        // [9..13]             -> size (4 bytes, big-endian)
        // [13..]              -> data (remaining bytes)
        Request {
            request_type: match buffer[0] {
                0 => RequestType::Abre,
                1 => RequestType::Le,
                2 => RequestType::Escreve,
                3 => RequestType::Fecha,
                _ => panic!("Invalid request type"),
            },
            descritor_arquivo: i32::from_be_bytes(buffer[1..5].try_into().unwrap()),
            posicao: u64::from_be_bytes(buffer[5..13].try_into().unwrap()),
            size: u32::from_be_bytes(buffer[13..17].try_into().unwrap()),
            data: buffer[17..].to_vec(),
        }
    }
    pub fn serialize(response: Request) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        buffer.push(response.request_type as u8);
        buffer.extend(&response.descritor_arquivo.to_be_bytes());
        buffer.extend(&response.posicao.to_be_bytes());
        buffer.extend(&response.size.to_be_bytes());
        buffer.extend(response.data);
        buffer
    }
}

impl Response {
    pub fn serialize(response: Response) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        buffer.push(response.response_type as u8);
        buffer.extend(response.data);
        buffer
    }
    pub fn desserialize(buffer: &Vec<u8>) -> Response {
        Response {
            response_type: match buffer[0] {
                0 => ResponseType::Ok,
                1 => ResponseType::AtualizaCache,
                2 => ResponseType::Erro,
                _ => panic!("Invalid response type"),
            },
            data: buffer[1..].to_vec(),
        }
    }
}