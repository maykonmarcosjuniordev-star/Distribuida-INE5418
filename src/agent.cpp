#include "../remote_file_system/src/client.cpp"


class Agent {
public:
    Client cliente;
    unsigned int id;

    Agent(const string server, unsigned int server_port, const string client, unsigned int client_port) {
        cliente = Client(server, server_port, client, client_port);
        id = 0;
    }

    void run() {
        printf("Agent %i started\n", id);
        int i = 1;
        while (i--) {
            string nome_arquivo = "file.txt";
            int id_file = 0;
            int posicao = 0;
            string text = "Hello from agent cpp\n";
            vector<char> buffer(text.begin(), text.end());
            unsigned int tamanho = buffer.size()*4;

            printf("Agent %i trying to open file %s\n", id, nome_arquivo.c_str());
            if (cliente.abre(id_file, nome_arquivo) == 0) {
                printf("Agent %i opened file %s\n", id, nome_arquivo.c_str());
            } else {
                printf("Error, couldn't open file.\n");
                continue;
            }
            printf("Agent %i writing to file %i: %s\n", id, id_file, text.c_str());
            if (cliente.escreve(id_file, 0, buffer, buffer.size()) == 0) {
                printf("Written!\n");
            } else {
                printf("Error, couldn't write to file.\n");
                continue;
            }
            printf("Agent %i reading from file %i\n", id, id_file);
            buffer.clear();
            if (int bytes = cliente.le(id_file, 0, buffer, 50) >= 0) {
                string readed(buffer.begin(), buffer.end());
                printf("Agent %i read %i bytes from file %i: \n", id, bytes, id_file);
                cout << "Read: " << readed << endl;
            } else {
                printf("Error, couldn't read file.\n");
                continue;
            }
            if (cliente.fecha(id_file) == 0) {
                printf("Agent closed file %i\n", id_file);
                break;
            } else {
                printf("Error, couldn't close file.\n");
            }
        }
    }
};





int main(int argc, char* argv[]) {
    Agent agente = Agent("127.0.0.1", 8080, "127.0.0.1", 8070);
    agente.run();
}
