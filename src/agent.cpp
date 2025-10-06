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
            string text = "Hello from agent " + to_string(id) + "\n";
            vector<char> buffer(text.begin(), text.end());

            if (cliente.abre(id_file, nome_arquivo) == 0) {
                printf("Agent %i opened file %s", id, nome_arquivo.c_str());
            } else {
                printf("Error, couldn't open file.\n");
                continue;
            }
            if (cliente.escreve(id_file, 0, buffer, buffer.size()) == 0) {
                printf("Written!");
            } else {
                printf("Something went wrong.");
                continue;
            }
            int result = cliente.le(id_file, 0, buffer, 1024);
            string readed(buffer.begin(), buffer.end());
            if (result >= 0) {
                printf("Agent %i read %i bytes from file %i: %s", id, result, id_file, readed.c_str());
            } else {
                printf("Error, couldn't read file.");
                continue;
            }
            if (cliente.fecha(id_file) == 0) {
                printf("Agent closed file %i", id_file);
                break;
            } else {
                printf("Error, couldn't read file.");
            }
        }
    }
};





int main(int argc, char* argv[]) {
    Agent agente = Agent("127.0.0.1", 8080, "127.0.0.1", 8081);
    agente.run();
}
