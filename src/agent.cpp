#include "../remote_file_system/src/client.cpp"


class Agent {
public:
    Client cliente;
    unsigned int id;

    Agent(unsigned int Porta, const string server) {
        cliente = Client(Porta, server);
        id = 0;
    }

    void run() {
        printf("Agent %i started", id);

        while (true) {
            printf("Type 'write', 'read', 'open', 'close', or 'exit' to quit.\n");

            string inpt;
            getline(cin, inpt);


            if (inpt == "write") {
                printf("Agent %i preparing to write. Enter text to write:\n", id);
                string text;
                getline(cin, text);

                printf("File id: ");
                int id_file;
                scanf("%d", &id_file);

                vector<char> buffer(text.begin(), text.end());
                int result = cliente.escreve(id_file, 0, buffer, buffer.size());
                if (result == 0) {
                    printf("Written!");
                } else {
                    printf("Something went wrong.");
                }
            } else if (inpt == "read") {
                printf("Id of File: ");
                int id_file;
                scanf("%d", &id_file);
                vector<char> buffer;
                int result = cliente.le(id_file, 0, buffer, 1024);
                string text(buffer.begin(), buffer.end());
                if (result >= 0) {
                    printf("Agent %i read %i bytes from file %i: %s", id, result, id_file, text.c_str());
                } else {
                    printf("Error, couldn't read file.");
                }
            } else if (inpt == "open") {
                printf("File name to open:\n");
                string nome;
                getline(cin, nome);

                printf("Id of File: ");
                int id_file;
                scanf("%d", &id_file);
                int result = cliente.abre(id_file, nome);
                if (result == 0) {
                    printf("Agent %i opened file %s", id, nome.c_str());
                } else {
                    printf("Error, couldn't read file.\n");
                }
            } else if (inpt == "close") {
                printf("File id to close: ");
                int id_file;
                scanf("%d", &id_file);
                int result = cliente.fecha(id_file);
                if (result == 0) {
                    printf("Agent closed file %i", id_file);
                } else {
                    printf("Error, couldn't read file.");
                }
                
            } else if (inpt == "exit") {
                printf("Goodbye");
                break;
            }
        }
    }
};





int main(int argc, char* argv[]) {
    Agent agente = Agent(8080, "127.0.0.1");
    agente.run();
}