use std::fs::{File, OpenOptions};
// use std::io::Read;
use std::collections::HashMap;
use std::os::unix::fs::FileExt;

pub struct FileManager {
    file_table: HashMap<u32, String>,
}

impl FileManager {
    pub fn new() -> Self {
        Self {file_table: HashMap::new()}
    }
    /// ● A função retorno um descritor de arquivo uma vez passado o nome do
    /// arquivo a ser aberto. Caso o arquivo não exista, ele será criado. Caso
    /// contrário, o descritor referenciar um arquivo já existente e este poderá sobre
    /// modificações ou ser lido;
    /// 
    /// ● o valor de retorno inteiro (int) deve representar códigos de erro, na
    /// impossibilidade de execução da operação;
    pub fn abre(&self, descritor_arquivo: u32, nome_arquivo: String) -> i32 {
        // verify if it is already on the table
        if self.file_table.contains_key(&descritor_arquivo) {
            return 0;
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
    pub fn le(&self, descritor_arquivo: u32, posicao: u64, buffer: &mut Vec<u8>, tamanho: usize) -> i32 {
        match self.file_table.get(&descritor_arquivo) {
            Some(nome_arquivo) => {
                let file = match OpenOptions::new().read(true).open(&nome_arquivo) {
                    Ok(f) => {f}
                    Err(e) => {
                        println!("Erro {} ao abrir arquivo: {}", e, nome_arquivo);
                        return -1;
                    }                    
                };
                match  file.read_exact_at(buffer[..tamanho].as_mut(), posicao) {
                    Ok(_) => {
                        if buffer.len() == tamanho {
                            tamanho as i32
                        } else {
                            buffer.len() as i32
                        }
                    }
                    Err(e) => {
                        println!("Erro {} ao abrir arquivo: {}", e, nome_arquivo);
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
    pub fn escreve(&self, descritor_arquivo: u32, posicao: u64, buffer: &mut Vec<u8>, tamanho: usize) -> i32 {
        match self.file_table.get(&descritor_arquivo) {
            Some(nome_arquivo) => {
                let file = match OpenOptions::new().read(true).open(&nome_arquivo) {
                    Ok(f) => {f}
                    Err(e) => {
                        println!("Erro {} ao abrir arquivo: {}", e, nome_arquivo);
                        return -1;
                    }                    
                };
                match  file.write_all_at(buffer[..tamanho].as_mut(), posicao) {
                    Ok(_) => {
                        if buffer.len() == tamanho {
                            tamanho as i32
                        } else {
                            buffer.len() as i32
                        }
                    }
                    Err(e) => {
                        println!("Erro {} ao abrir arquivo: {}", e, nome_arquivo);
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
    pub fn fecha(&self, descritor_arquivo: u32) -> i32{
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
        0
    }
}
