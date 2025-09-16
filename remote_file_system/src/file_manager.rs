use std::fs::{File, OpenOptions};
use std::io;
use std::collections::HashMap;

pub struct FileManager {
    file_table: HashMap<i32, String>,
}

impl FileManager {
    /// ● A função retorno um descritor de arquivo uma vez passado o nome do
    /// arquivo a ser aberto. Caso o arquivo não exista, ele será criado. Caso
    /// contrário, o descritor referenciar um arquivo já existente e este poderá sobre
    /// modificações ou ser lido;
    /// 
    /// ● o valor de retorno inteiro (int) deve representar códigos de erro, na
    /// impossibilidade de execução da operação;
    pub fn abre(&self, descritor_arquivo: i32, nome_arquivo: String) -> i32 {
        // verify if it is already on the table
        if self.file_table.contains_key(&descritor_arquivo) {
            return descritor_arquivo;
        }
        // TODO: usar lifetime para permitir que o arquivo saia de escopo
        let _file: File = match OpenOptions::new()
        .create(true)
        .open(&nome_arquivo) {
            Ok(f) => f,
            Err(e) => {
                println!("Erro {} ao abrir arquivo: {}", e, nome_arquivo);
                return -1;
            },
        };
        descritor_arquivo
    }
    /// ● descritor_arquivo indica o identificador do descritor ao qual se pretende manipular;
    /// 
    /// ● posicao indica a posição inicial do arquivo de onde se pretende ler algum conteúdo;
    /// 
    /// ● buffer indica o endereço da variável que receberá o conteúdo da leitura;
    /// 
    /// ● tamanho indica o número em bytes a serem lidos na operação
    ///     (ou seja, o número de bytes a partir da posição posicao);
    /// 
    /// ● o valor de retorno inteiro (int) deve representar códigos de erro,
    ///     na impossibilidade de execução da operação.
    pub fn le(&self, descritor_arquivo: i32, posicao: u32, buffer: &Vec<u8>, tamanho: u32) -> i32 {
        
        -1
    }
    /// ● descritor_arquivo indica o identificador do descritor ao qual se
    ///     pretende manipular;
    /// 
    /// ● posicao indica a posição inicial do arquivo onde se pretende escrever
    ///     algum conteúdo;
    /// 
    /// ● buffer indica o endereço da variável que contém o conteúdo a ser escrito
    ///     no espaço de endereçamento
    /// 
    /// ● tamanho indica o número em bytes a serem escritos na operação
    ///     (ou seja, o número de bytes em buffer, a partir da posição posicao);
    /// 
    /// ● o valor de retorno inteiro (int) deve representar códigos de erro,
    ///     na impossibilidade de execução da operação.
    pub fn escreve(&self, descritor_arquivo: i32, posicao: u32, buffer: &Vec<u8>, tamanho: u32) -> i32 {
        -1
    }
    /// ● descritor_arquivo indica o identificador do descritor de arquivo a ser fechado.
    pub fn fecha(&self, descritor_arquivo: i32) -> i32{
        if !self.file_table.contains_key(&descritor_arquivo) {
            return -1;
        }
        let nome_arquivo = self.file_table.get(&descritor_arquivo).expect("shouldn't happen");
        // fecha o arquivo
        let file = match OpenOptions::new()
        .open(&nome_arquivo) {
            Ok(f) => f,
            Err(e) => {
                println!("Erro {} ao fechar arquivo: {}", e, nome_arquivo);
                return -1;
            },
        };
        drop(file);
        descritor_arquivo
    }
}
