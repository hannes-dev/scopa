use std::{collections::HashMap, net::Ipv4Addr};

#[derive(Debug, PartialEq, Eq)]
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

#[derive(Debug, PartialEq, Eq)]
struct Question {
    domain_name: Vec<String>,
    q_type: u16,
    q_class: u16,
}

#[derive(Debug, PartialEq, Eq)]
struct ResourceRecord {
    name: Vec<String>,
    r#type: u16,
    class: u16,
    ttl: u32,
    data_length: u16,
    data: ResourceData,
}

#[derive(Debug, PartialEq, Eq)]
enum ResourceData {
    A(Ipv4Addr),
}

#[derive(Debug, PartialEq, Eq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_question() {
        let buf = [141, 225, 1, 32, 0, 1, 0, 0, 0, 0, 0, 0, 7, 101, 120, 97, 109, 112, 108, 101, 3, 99, 111, 109, 0, 0, 1, 0, 1];
        let parsed_message = parse_message(&buf);

        let expected_message = Message {
            header: Header { id: 36321, q_type: 0, truncated: false, recursion_desired: true, question_count: 1, answer_count: 0, authority_count: 0, additional_count: 0 },
            question: Question { domain_name: ["example".to_string(), "com".to_string()].into(), q_type: 1, q_class: 1 },
            answers: None,
        };

        assert_eq!(expected_message.header, parsed_message.header);
        assert_eq!(expected_message.question, parsed_message.question);
        assert_eq!(expected_message.answers, parsed_message.answers);
    }

    #[test]
    fn test_parse_response() {
        let buf = [141, 225, 129, 160, 0, 1, 0, 1, 0, 0, 0, 0, 7, 101, 120, 97, 109, 112, 108, 101, 3, 99, 111, 109, 0, 0, 1, 0, 1, 192, 12, 0, 1, 0, 1, 0, 1, 42, 15, 0, 4, 93, 184, 216, 34];
        let parsed_message = parse_message(&buf);

        let domain_name = vec!["example".to_string(), "com".to_string()];
        let expected_message = Message {
            header: Header { id: 36321, q_type: 0, truncated: false, recursion_desired: true, question_count: 1, answer_count: 1, authority_count: 0, additional_count: 0 },
            question: Question { domain_name: domain_name.clone(), q_type: 1, q_class: 1 },
            answers: Some(vec![ResourceRecord { name: domain_name, r#type: 1, class: 1, ttl: 76303, data_length: 4, data: ResourceData::A(Ipv4Addr::from([93, 184, 216, 34])) }]),
        };

        assert_eq!(expected_message.header, parsed_message.header);
        assert_eq!(expected_message.question, parsed_message.question);
        assert_eq!(expected_message.answers, parsed_message.answers);
    }
}