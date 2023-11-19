use std::{collections::HashMap, net::Ipv4Addr};

#[derive(Debug)]
struct Header {
    id: u16,
    q_type: u8,
    truncated: bool,
    recursion_desired: bool,

    question_count: u16,
    answer_count: u16,
    authority_count: u16,
    additional_count: u16,
}

#[derive(Debug)]
struct Question {
    domain_name: Vec<String>,
    q_type: u16,
    q_class: u16,
}

#[derive(Debug)]
struct ResourceRecord {
    name: Vec<String>,
    r#type: u16,
    class: u16,
    ttl: u32,
    data_length: u16,
    data: ResourceData,
}

#[derive(Debug)]
enum ResourceData {
    A(Ipv4Addr),
}

#[derive(Debug)]
pub struct Message {
    header: Header,
    question: Question,
    answers: Option<Vec<ResourceRecord>>,
}

pub fn parse_message(buf: &[u8]) -> Message {
    let mut index = 0;
    let header = parse_header(&buf, &mut index);

    let mut names = HashMap::new();
    let question = parse_question(&mut names, &buf, &mut index);
    let answers = parse_resource_records(header.answer_count, &mut names, &buf, &mut index);

    Message {
        header,
        question,
        answers,
    }
}

fn parse_header(buf: &[u8], index: &mut usize) -> Header {
    *index = 12;

    Header {
        id: u16::from_be_bytes([buf[0], buf[1]]),
        // Query/response bit,
        q_type: (buf[2] >> 3) | 0b00001111,
        // Authorative Answer,
        truncated: bits_set(&buf[2], 0x02),
        recursion_desired: bits_set(&buf[2], 0x01),
        // Recursion available,
        // Zeros
        // Response code
        question_count: u16::from_be_bytes([buf[4], buf[5]]),
        answer_count: u16::from_be_bytes([buf[6], buf[7]]),
        authority_count: u16::from_be_bytes([buf[8], buf[9]]),
        additional_count: u16::from_be_bytes([buf[10], buf[11]]),
    }
}

fn parse_question(
    names: &mut HashMap<usize, Vec<String>>,
    buf: &[u8],
    index: &mut usize,
) -> Question {
    let domain_name = parse_name(names, buf, index);

    let q_type = u16::from_be_bytes([buf[*index], buf[*index + 1]]);
    *index += 2;
    let q_class = u16::from_be_bytes([buf[*index], buf[*index + 1]]);
    *index += 2;

    Question {
        domain_name,
        q_type,
        q_class,
    }
}

fn parse_resource_records(
    amt: u16,
    names: &mut HashMap<usize, Vec<String>>,
    buf: &[u8],
    index: &mut usize,
) -> Option<Vec<ResourceRecord>> {
    if amt == 0 {
        return None;
    }

    let mut resource_records = Vec::new();

    for _ in 0..amt {
        let name = parse_name(names, buf, index);
        let r#type = u16::from_be_bytes([buf[*index], buf[*index + 1]]);
        *index += 2;
        let class = u16::from_be_bytes([buf[*index], buf[*index + 1]]);
        *index += 2;
        let ttl = u32::from_be_bytes([
            buf[*index],
            buf[*index + 1],
            buf[*index + 2],
            buf[*index + 3],
        ]);
        *index += 4;
        let data_length = u16::from_be_bytes([buf[*index], buf[*index + 1]]);
        *index += 2;
        let data = match r#type {
            1 => ResourceData::A(Ipv4Addr::from([
                buf[*index],
                buf[*index + 1],
                buf[*index + 2],
                buf[*index + 3],
            ])),
            _ => ResourceData::A(Ipv4Addr::new(0, 0, 0, 0)),
        };
        *index += data_length as usize;
        resource_records.push(ResourceRecord {
            name,
            r#type,
            class,
            ttl,
            data_length,
            data,
        })
    }

    Some(resource_records)
}

fn parse_name(
    names: &mut HashMap<usize, Vec<String>>,
    buf: &[u8],
    index: &mut usize,
) -> Vec<String> {
    let mut name = Vec::new();
    let mut length = buf[*index] as usize;
    let offset = *index;
    *index += 1;

    while length > 0 {
        // pointer to another name
        if bits_set(&(length as u8), 0b11000000) {
            let offset = u16::from_be_bytes([length as u8 & 0b00111111, buf[*index]]) as usize;
            name.extend(names[&offset].clone());
            *index += 1;
            break;
        }

        let label = String::from_utf8_lossy(&buf[*index..*index + length]).to_string();
        name.push(label);

        *index += length;
        length = buf[*index] as usize;
        *index += 1;
    }

    if !name.is_empty() {
        names.insert(offset, name.clone());
    }

    name
}

fn bits_set(byte: &u8, bit_pos: u8) -> bool {
    byte & bit_pos == bit_pos
}