use std::fs::{File, OpenOptions};
use std::collections::HashMap;
use std::os::unix::fs::FileExt;
use std::sync::Mutex;
use crate::server_log;

const FILES_PATH: &str = "./files/";

pub struct FileManager {
    file_table: Mutex<HashMap<i32, File>>,
}

impl FileManager {
    pub fn new() -> Self {
        // Ensure the files directory exists
        if let Err(e) = std::fs::create_dir_all(FILES_PATH) {
            server_log!("Warning: Could not create files directory on {}: {}", FILES_PATH, e);
        }
        Self {file_table: Mutex::new(HashMap::new())}
    }
    /// ● A função retorno um descritor de arquivo uma vez passado o nome do
    /// arquivo a ser aberto. Caso o arquivo não exista, ele será criado. Caso
    /// contrário, o descritor referenciar um arquivo já existente e este poderá sobre
    /// modificações ou ser lido;
    /// 
    /// ● o valor de retorno inteiro (int) deve representar códigos de erro, na
    /// impossibilidade de execução da operação;
    pub fn abre(&self, descritor_arquivo: i32, nome_arquivo: &String) -> i32 {
        // verify if it is already on the table
        if self.file_table.lock().expect("Failed to lock file table").contains_key(&descritor_arquivo) {
            return 0;
        }
        // Clean the filename to remove any null bytes or invalid characters
        let clean_filename = nome_arquivo
            .trim_end_matches('\0')  // Remove trailing null bytes
            .replace('\0', "");      // Remove any internal null bytes
            
        let file_path = format!("{}{}", FILES_PATH, clean_filename);
        
        let file: File = match OpenOptions::new()
            .create(true)
            .read(true)     // Add read permission
            .write(true)    // Add write permission
        .open(file_path) {
            Ok(f) => f,
            Err(e) => {
                server_log!("Erro {} ao abrir arquivo: {}", e, nome_arquivo);
                return -1;
            },
        };
        self.file_table.lock().expect("Failed to lock file table").insert(descritor_arquivo, file);
        0
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
    pub fn le(&self, descritor_arquivo: i32, posicao: u64, buffer: &mut Vec<u8>, tamanho: usize) -> i32 {
        match self.file_table.lock().expect("Failed to lock file table").get(&descritor_arquivo) {
            Some(file) => {
                server_log!("Reading {} bytes from file {} at position {}", tamanho, descritor_arquivo, posicao);
                match  file.read_at(buffer[..tamanho].as_mut(), posicao) {
                    Ok(size) => {
                        size as i32
                    }
                    Err(e) => {
                        server_log!("Erro ao ler arquivo {}: {}", descritor_arquivo, e);
                        -1
                    }
                }
            }
            None => {
                -1
            }
        }
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
    pub fn escreve(&self, descritor_arquivo: i32, posicao: u64, buffer: &mut Vec<u8>, tamanho: usize) -> i32 {
        match self.file_table.lock().expect("Failed to lock file table").get(&descritor_arquivo) {
            Some(file) => {
                match  file.write_all_at(buffer[..tamanho].as_mut(), posicao) {
                    Ok(_) => {
                        buffer.len() as i32
                    }
                    Err(e) => {
                        server_log!("Erro {} ao abrir arquivo: {}", e, descritor_arquivo);
                        -1
                    }
                }
            }
            None => {
                -1
            }
        }
    }
    /// ● descritor_arquivo indica o identificador do descritor de arquivo a ser fechado.
    /// Retorna 0 em caso de sucesso e -1 em caso de erro.
    pub fn fecha(&self, descritor_arquivo: i32) -> i32{
        if !self.file_table.lock().expect("Failed to lock file table").contains_key(&descritor_arquivo) {
            server_log!("Arquivo {} não encontrado para fechar", descritor_arquivo);
            return -1;
        }
        // fecha o arquivo
        match self.file_table.lock().expect("Failed to lock file table").remove(&descritor_arquivo) {
            Some(f) => {
                drop(f);
                0
            },
            None => -1,
        }
    }
}
