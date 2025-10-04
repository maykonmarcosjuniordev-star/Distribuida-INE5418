#include <sys/socket.h>
#include <arpa/inet.h>
#include <vector>
#include <string>
#include <cstdint>
#include <stdexcept>
#include <unistd.h>
#include <sys/select.h>


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
    int posicao;
    int size;
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
    int client_socket;

    void setup() {
        client_socket = socket(AF_INET, SOCK_STREAM, 0);
        server_address.sin_family = AF_INET;
        server_address.sin_port = htons(8080); // E a porta?
        server_address.sin_addr.s_addr = inet_addr("127.0.0.1"); // Como fazer o IP do servidor?
    }

    int sendServer(vector<char> buffer, struct Request rqst) {
        int result;
        int len = sizeof(server_address);
        result = connect(client_socket, (struct sockaddr *)&server_address, len);
        if (result == -1) {
            perror("AAAAAAAAAAAAAAAAAAAAAAAAA"); // TODO
            return -1;
        }

        write(client_socket, &rqst, sizeof(rqst));
        read(client_socket, &buffer, BUFFER_SIZE);

        close(client_socket);
    }

    int abre(int descritor_arquivo, string nome_arquivo) {
        struct Request rqst;
        rqst.request_type = RequestType::ABRE;
        rqst.descritor_arquivo = descritor_arquivo;
        rqst.posicao = 0;
        rqst.size = 0;
        vector<char> data(nome_arquivo.begin(), nome_arquivo.end());
        rqst.data = data;
        
        
        vector<char> buffer;
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
        
        
        vector<char> buffer;
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


        sendServer(buffer, rqst);

        struct Response rsp = desserialize(buffer);
        return rsp.response_type;
    }

    int le(int descritor_arquivo, unsigned long long posicao, vector<char> &buffer, int tamanho) {
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


    int verify_cache(CacheItem checkItem) {
        int result;
        result = connect(client_socket, (struct sockaddr *)&server_address, sizeof(server_address));
        if (result == -1) {
            perror("AAAAAAAAAAAAAAAAAAAAAAAAA"); // TODO
            return -1;
        }
        
        
        fd_set readfds;
        FD_ZERO(&readfds);
        FD_SET(client_socket, &readfds);
        struct timeval timeout;
        timeout.tv_sec = 0;
        timeout.tv_usec = 0;

        int activity = select(client_socket + 1, &readfds, NULL, NULL, &timeout);

        int invalidated = 0;
        vector<char> buffer;
        while (activity > 0 && FD_ISSET(client_socket, &readfds)) {
            read(client_socket, &buffer, 24);
            struct Response rsp = desserialize(buffer);

            if (rsp.response_type == CACHE_UPDATE) {
                vector<char> data = rsp.data;
                int descriptor = getNumberi32(data);
                int posicao = getNumberu64(data);
                int sizee = getNumberu64(data);
                
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
            }


            int activity = select(client_socket + 1, &readfds, NULL, NULL, &timeout);
        }
        return invalidated;
    }



    // int serialize(struct Request &rqst) { // Do we need this?
    //     int32_t byte0 = htonl(rqst.request_type);
    //     int32_t byte4 = htonl(rqst.descritor_arquivo);
    //     int32_t byte8 = htonl(rqst.posicao);
    //     int32_t byte12 = htonl(rqst.size);
    //     int32_t byte16e = htonl((int32_t)rqst.data.size());
    // }

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

};


// int main() {

// }
