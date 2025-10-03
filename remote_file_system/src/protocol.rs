pub const BUFFER_SIZE: usize = 1024;

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RequestType {
    Abre = 0,
    Le = 1,
    Escreve = 2,
    Fecha = 3,
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ResponseType {
    Ok = 0,
    AtualizaCache = 1,
    Erro = 2,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Request {
    pub request_type: RequestType,
    pub descritor_arquivo: i32,
    pub posicao: u64,
    pub size: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Response {
    /// 1 -> Atualiza cache dos clientes devido a write
    pub response_type: ResponseType,
    pub data: Vec<u8>,
}

impl Request {
    /// Desserializa o buffer recebido do cliente
    pub fn desserialize(buffer: &Vec<u8>) -> Request {
        // Expected layout:
        // [0]                 -> request_type (1 byte)
        // [1..5]              -> descritor_arquivo (4 bytes, big-endian)
        // [5..9]              -> posicao (4 bytes, big-endian)
        // [9..13]             -> size (4 bytes, big-endian)
        // [13..]              -> data (remaining bytes)
        let request_type = match buffer[0] {
            0 => RequestType::Abre,
            1 => RequestType::Le,
            2 => RequestType::Escreve,
            3 => RequestType::Fecha,
            _ => panic!("Invalid request type"),
        };
        let descritor_arquivo = i32::from_be_bytes(buffer[1..5].try_into().unwrap());
        let posicao = u64::from_be_bytes(buffer[5..13].try_into().unwrap());
        let size = u32::from_be_bytes(buffer[13..17].try_into().unwrap());
        let data = buffer[17..(17 + (size as usize))].to_vec();
        Request {
            request_type,
            descritor_arquivo,
            posicao,
            size,
            data,
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
    pub fn create_atualiza_cache(descritor_arquivo: i32,
            posicao: u64, tamanho: usize) -> Response {
        let mut data = descritor_arquivo.to_be_bytes().to_vec();
        data.extend(posicao.to_be_bytes().to_vec());
        data.extend((tamanho as u64).to_be_bytes().to_vec());
        Response {
            response_type: ResponseType::AtualizaCache,
            data,
        }
    }
    pub fn parse_atualiza_cache(response: Response) -> (i32, u64, usize) {
        let descritor_arquivo = i32::from_be_bytes(response.data[0..4].try_into().unwrap());
        let posicao = u64::from_be_bytes(response.data[4..12].try_into().unwrap());
        let tamanho = u64::from_be_bytes(response.data[12..20].try_into().unwrap()) as usize;
        (descritor_arquivo, posicao, tamanho)
    }
}
