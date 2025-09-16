/*
Coerência de cache
As operações de leitura e de escrita podem acessar partes do arquivo (blocos)
localmente ou remotamente. Por exemplo, uma leitura de regiões previamente carregadas
na cache local, retornam diretamente para a função de leitura, sem gerar requisições ao
servidor. Para isso, os processos solicitantes devem fazer cópias do(s) bloco(s) de interesse
e mantê-la em uma cache local quando da escrita ou leitura desses conteúdos.
No caso de leituras e leituras sucessivas, se o processo tiver uma cópia válida do bloco
de interesse, basta ler da sua cache. No caso de escritas, o processo deve atualizar o valor
do bloco no arquivo em questão e gerar uma invalidação das cópias daquele bloco em
caches de outros processos. Este procedimento é comum na implementação de
mecanismos para coerência de cache e chama-se invalidação na escrita. É permitida a
utilização de outras estratégias para coerência de cache.
*/

use crate::server::Server

pub struct  Client {
    cache: Vec<u8>,
    server: Server,
}

impl Client {
    pub fn new(server: Server) -> Self {
        let cache = Vec::new();
        Self {server, cache}
    }
    pub fn abre(&self, descritor_arquivo: i32, nome_arquivo: String) -> i32 {
        if self.cache.len() == 0 {
            -1
        } else {
            -1
        }
    }
}
