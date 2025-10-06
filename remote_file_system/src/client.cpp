#include <sys/socket.h>
#include <sys/select.h>
#include <sys/un.h>
#include <arpa/inet.h>
#include <stdlib.h>
#include <unistd.h>

#include <vector>
#include <string>
#include <cstdint>
#include <stdexcept>
#include <iostream>


using namespace std;


const int MAX_CACHE_SIZE = 1024;
const int BUFFER_SIZE = 1024;

struct CacheItem {
    int descritor_arquivo; // i32
    long long start; // u64
    long long end; // u64
    vector<char> data; // u8
};

enum RequestType {
    ABRE,
    LE,
    ESCREVE,
    FECHA,
};
enum ResponseType {
    OK,
    CACHE_UPDATE,
    ERRO,
};

struct Request {
    enum RequestType request_type;
    int descritor_arquivo;
    unsigned long long posicao;
    unsigned int size;
    vector<char> data;
};

struct Response {
    enum ResponseType response_type;
    vector<char> data;
};

class Client {
public:
    vector<CacheItem> cache;
    struct sockaddr_in server_address;
    struct sockaddr_in client_address;
    int client_socket;
    int warning_socket;
    
    Client() {}
    
    Client(const string server, unsigned int server_port, const string client, unsigned int client_port) {
        client_socket = socket(AF_INET, SOCK_STREAM, 0);
        // binds client socket to client address
        client_address.sin_family = AF_INET;
        client_address.sin_port = htons(client_port);
        bind(client_socket, (struct sockaddr*)&client_address, sizeof(client_address));
        warning_socket = socket(AF_INET, SOCK_DGRAM, 0);
        // binds warning socket to client address
        bind(warning_socket, (struct sockaddr*)&client_address, sizeof(client_address));
        // set timeout for warning socket
        struct timeval timeout;
        timeout.tv_sec = 0;
        timeout.tv_usec = 5; // 5 us = 5 ms
        setsockopt(warning_socket, SOL_SOCKET, SO_RCVTIMEO, (const char*)&timeout, sizeof(timeout));
        // configure server address
        server_address.sin_family = AF_INET;
        server_address.sin_port = htons(server_port);
        server_address.sin_addr.s_addr = inet_addr(server.c_str());
    }

    int sendServer(vector<char> &buffer, struct Request rqst) {
        int result;
        int len = sizeof(server_address);

        // connect to server
        result = connect(client_socket, (struct sockaddr *) &server_address, len);
        if (result < 0) {
            perror("Connection to server failed");
            return -1;
        }
        
        vector<char> message = serialize(rqst);
        // converts to string and prints
        printf("Sending message to server: ");
        for (char c : message) {
            printf("%c", c);
        }
        printf("\n");
        write(client_socket, message.data(), message.size());
        
        buffer.resize(BUFFER_SIZE);

        socklen_t recv_len = sizeof(server_address);
        printf("Waiting for response from server...\n");
        read(client_socket, buffer.data(), BUFFER_SIZE);
        printf("Response received from server.\n");

        return 0;
    }

    int abre(int descritor_arquivo, string nome_arquivo) {
        struct Request rqst;
        rqst.request_type = RequestType::ABRE;
        rqst.descritor_arquivo = descritor_arquivo;
        rqst.posicao = 0;
        rqst.size = 0;
        vector<char> data(nome_arquivo.begin(), nome_arquivo.end());
        rqst.data = data;
        // reconverts data to string and prints
        
        vector<char> buffer;
        int result = sendServer(buffer, rqst);

        if (result == -1) {
            return result;
        }

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
        
        
        vector<char> buffer;
        int result = sendServer(buffer, rqst);

        if (result == -1) {
            return result;
        }

        Response rsp = desserialize(buffer);
        return rsp.response_type;
    }

    int escreve(int descritor_arquivo, unsigned long long posicao, vector<char> &buffer, unsigned int tamanho) {
        Request rqst;
        rqst.request_type = RequestType::ESCREVE;
        rqst.descritor_arquivo = descritor_arquivo;
        rqst.posicao = posicao;
        rqst.size = tamanho;
        rqst.data = buffer;

        int result = sendServer(buffer, rqst);
        
        if (result == -1) {
            return result;
        }
        
        struct Response rsp = desserialize(buffer);
        return rsp.response_type;
    }

    int le(int descritor_arquivo, unsigned long long posicao, vector<char> &buffer, unsigned int tamanho) {
        for (auto item : cache) {
            if (item.descritor_arquivo == descritor_arquivo and item.start <= posicao and posicao+tamanho <= item.end) {
                int cacheStatus = verify_cache(item);
                if (cacheStatus == 0) {
                    int start = posicao - item.start;
                    int end = start+tamanho;
                    copy(item.data.begin(), item.data.end(), back_inserter(buffer));
                    return tamanho;
                }
                if (cacheStatus == -1) {
                    return -1; // ERRO!!
                }
            }
        }

        Request rqst;
        rqst.request_type = RequestType::LE;
        rqst.descritor_arquivo = descritor_arquivo;
        rqst.posicao = posicao;
        rqst.size = tamanho;
        rqst.data = {};

        int result = sendServer(buffer, rqst);

        if (result == -1) {
            return result;
        }

        Response rsp = desserialize(buffer);

        if (rsp.response_type == ResponseType::OK) {
            CacheItem item;
            item.descritor_arquivo = descritor_arquivo;
            item.start = posicao;
            item.end = posicao+tamanho;
            item.data = rsp.data;
            cache.push_back(item);
            // garantee cache size
            if (cache.size() > MAX_CACHE_SIZE) {
                cache.erase(cache.begin());
            }
            return tamanho;
        } else { // ERROR
            return -1;
        }
    }


    int verify_cache(CacheItem checkItem) {
        socklen_t recv_len = sizeof(server_address);
        vector<char> buffer;
        int invalidated = 0;
        while (true) {
            recvfrom(warning_socket, buffer.data(), 24, MSG_DONTWAIT, (struct sockaddr *) &server_address, &recv_len);
            if (buffer.size() == 0) {
                break;
            }
            struct Response rsp = desserialize(buffer);

            if (rsp.response_type == CACHE_UPDATE) {
                vector<char> data = rsp.data;
                int descriptor = getNumberi32(data);
                unsigned long long posicao = getNumberu64(data);
                unsigned int sizee = getNumberu32(data);
                
                if ((checkItem.descritor_arquivo == descriptor) && (
                    (checkItem.start >= posicao && posicao <= checkItem.end) ||
                    (checkItem.start >= posicao + sizee && posicao + sizee <= checkItem.end))) {
                    invalidated = 1;
                }
                
                erase_if(cache, [&](const CacheItem& item) {
                    return (item.descritor_arquivo == descriptor) &&
                        (
                            (item.start >= posicao && posicao <= item.end) ||
                            (item.start >= posicao + sizee && posicao + sizee <= item.end)
                        );
                });
            } else {
                cout << "Mensagem inesperada recebida durante verificação da cache: " << rsp.response_type << endl;
                return -1;
            }
        }
        return invalidated;
    }




    vector<char> serialize(Request &rqst) {
        vector<char> buffer;

        // lambda sinistro
        auto appendBytes = [&](auto value) {
            using T = decltype(value);
            for (size_t i = 0; i < sizeof(T); ++i)
                buffer.push_back(static_cast<char>((value >> (8 * (sizeof(T) - 1 - i))) & 0xFF));
        };

        appendBytes(rqst.request_type);
        appendBytes(rqst.descritor_arquivo);
        appendBytes(rqst.posicao);
        appendBytes(rqst.size);
        buffer.insert(buffer.end(), rqst.data.begin(), rqst.data.end());
        
        return buffer;
    }

    struct Response desserialize(vector<char> buffer) {
        struct Response rsp;
        uint32_t b0 = static_cast<uint8_t>(buffer[0]);
        uint32_t b1 = static_cast<uint8_t>(buffer[1]);
        uint32_t b2 = static_cast<uint8_t>(buffer[2]);
        uint32_t b3 = static_cast<uint8_t>(buffer[3]);
        int type = (b0 << 24) | (b1 << 16) | (b2 << 8) | b3;
        rsp.response_type = static_cast<ResponseType>(type);
        copy(buffer.begin() + 4, buffer.end(), back_inserter(rsp.data));
        return rsp;
    }

    int getNumberi32(vector<char> &buffer) {
        uint32_t b0 = static_cast<uint8_t>(buffer[0]);
        uint32_t b1 = static_cast<uint8_t>(buffer[1]);
        uint32_t b2 = static_cast<uint8_t>(buffer[2]);
        uint32_t b3 = static_cast<uint8_t>(buffer[3]);
        int number = (b0 << 24) | (b1 << 16) | (b2 << 8) | b3;
        buffer.erase(buffer.begin(), buffer.begin() + 4);
        return number;
    }

    unsigned long long getNumberu64(vector<char> &buffer) {
        uint64_t b0 = static_cast<uint8_t>(buffer[0]);
        uint64_t b1 = static_cast<uint8_t>(buffer[1]);
        uint64_t b2 = static_cast<uint8_t>(buffer[2]);
        uint64_t b3 = static_cast<uint8_t>(buffer[3]);
        uint64_t b4 = static_cast<uint8_t>(buffer[4]);
        uint64_t b5 = static_cast<uint8_t>(buffer[5]);
        uint64_t b6 = static_cast<uint8_t>(buffer[6]);
        uint64_t b7 = static_cast<uint8_t>(buffer[7]);
        int number = (b0 << 56) | (b1 << 48) | (b2 << 40) | (b3 << 32) | (b4 << 24) | (b5 << 16) | (b6 << 8) | b7;
        buffer.erase(buffer.begin(), buffer.begin() + 8);
        return number;
    }

    unsigned int getNumberu32(vector<char> &buffer) {
        uint32_t b0 = static_cast<uint8_t>(buffer[0]);
        uint32_t b1 = static_cast<uint8_t>(buffer[1]);
        uint32_t b2 = static_cast<uint8_t>(buffer[2]);
        uint32_t b3 = static_cast<uint8_t>(buffer[3]);
        unsigned int number = (b0 << 24) | (b1 << 16) | (b2 << 8) | b3;
        buffer.erase(buffer.begin(), buffer.begin() + 4);
        return number;
    }
};

