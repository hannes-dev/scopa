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

struct Buffer<'a> {
    next_index: usize,
    buffer: &'a [u8],
}

impl Buffer<'_> {
    fn new(buf: &[u8]) -> Buffer {
        Buffer {
            next_index: 0,
            buffer: buf,
        }
    }

    fn next(&mut self) -> u8 {
        let curr_index = self.next_index;
        self.next_index += 1;

        self.buffer[curr_index]
    }

    fn next_n(&mut self, n: usize) -> &[u8] {
        let start = self.next_index;
        self.next_index += n;

        &self.buffer[start..self.next_index]
    }
}

pub fn parse_message(buf: &[u8]) -> Message {
    let buf = &mut Buffer::new(buf);
    let header = parse_header(buf);

    let mut names = HashMap::new();
    let question = parse_question(buf, &mut names);
    let answers = parse_resource_records(buf, header.answer_count, &mut names);

    Message {
        header,
        question,
        answers,
    }
}

fn parse_header(buf: &mut Buffer) -> Header {
    let id = u16::from_be_bytes([buf.next(), buf.next()]);

    let flag_byte = buf.next();
    // Query/response bit
    let q_type = (flag_byte >> 3) & 0b00001111;
    // Authorative Answer,
    let truncated = bits_set(flag_byte, 0x02);
    let recursion_desired = bits_set(flag_byte, 0x01);

    buf.next();
    // Recursion available,
    // Zeros
    // Response code

    Header {
        id,
        q_type,
        truncated,
        recursion_desired,
        question_count: u16::from_be_bytes([buf.next(), buf.next()]),
        answer_count: u16::from_be_bytes([buf.next(), buf.next()]),
        authority_count: u16::from_be_bytes([buf.next(), buf.next()]),
        additional_count: u16::from_be_bytes([buf.next(), buf.next()]),
    }
}

fn parse_question(buf: &mut Buffer, names: &mut HashMap<usize, Vec<String>>) -> Question {
    let domain_name = parse_name(buf, names);

    let q_type = u16::from_be_bytes([buf.next(), buf.next()]);
    let q_class = u16::from_be_bytes([buf.next(), buf.next()]);

    Question {
        domain_name,
        q_type,
        q_class,
    }
}

fn parse_resource_records(
    buf: &mut Buffer,
    amt: u16,
    names: &mut HashMap<usize, Vec<String>>,
) -> Option<Vec<ResourceRecord>> {
    if amt == 0 {
        return None;
    }

    let mut resource_records = Vec::new();

    for _ in 0..amt {
        let name = parse_name(buf, names);
        let r#type = u16::from_be_bytes([buf.next(), buf.next()]);
        let class = u16::from_be_bytes([buf.next(), buf.next()]);
        let ttl = u32::from_be_bytes([buf.next(), buf.next(), buf.next(), buf.next()]);
        let data_length = u16::from_be_bytes([buf.next(), buf.next()]);
        let data = match r#type {
            1 => ResourceData::A(Ipv4Addr::from([
                buf.next(),
                buf.next(),
                buf.next(),
                buf.next(),
            ])),
            _ => ResourceData::A(Ipv4Addr::new(0, 0, 0, 0)),
        };

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

fn parse_name(buf: &mut Buffer, names: &mut HashMap<usize, Vec<String>>) -> Vec<String> {
    let mut name = Vec::new();
    let offset = buf.next_index;
    let mut length = buf.next();

    while length > 0 {
        // pointer to another name
        if bits_set(length, 0b11000000) {
            let offset = u16::from_be_bytes([length & 0b00111111, buf.next()]) as usize;
            name.extend(names[&offset].clone());
            break;
        }

        let label = String::from_utf8_lossy(&buf.next_n(length as usize)).to_string();
        name.push(label);

        length = buf.next();
    }

    if !name.is_empty() {
        names.insert(offset, name.clone());
    }

    name
}

fn bits_set(byte: u8, bit_pos: u8) -> bool {
    byte & bit_pos == bit_pos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_question() {
        let buf = [
            141, 225, 1, 32, 0, 1, 0, 0, 0, 0, 0, 0, 7, 101, 120, 97, 109, 112, 108, 101, 3, 99,
            111, 109, 0, 0, 1, 0, 1,
        ];
        let parsed_message = parse_message(&buf);

        let expected_message = Message {
            header: Header {
                id: 36321,
                q_type: 0,
                truncated: false,
                recursion_desired: true,
                question_count: 1,
                answer_count: 0,
                authority_count: 0,
                additional_count: 0,
            },
            question: Question {
                domain_name: ["example".to_string(), "com".to_string()].into(),
                q_type: 1,
                q_class: 1,
            },
            answers: None,
        };

        assert_eq!(expected_message.header, parsed_message.header);
        assert_eq!(expected_message.question, parsed_message.question);
        assert_eq!(expected_message.answers, parsed_message.answers);
    }

    #[test]
    fn test_parse_response() {
        let buf = [
            141, 225, 129, 160, 0, 1, 0, 1, 0, 0, 0, 0, 7, 101, 120, 97, 109, 112, 108, 101, 3, 99,
            111, 109, 0, 0, 1, 0, 1, 192, 12, 0, 1, 0, 1, 0, 1, 42, 15, 0, 4, 93, 184, 216, 34,
        ];
        let parsed_message = parse_message(&buf);

        let domain_name = vec!["example".to_string(), "com".to_string()];
        let expected_message = Message {
            header: Header {
                id: 36321,
                q_type: 0,
                truncated: false,
                recursion_desired: true,
                question_count: 1,
                answer_count: 1,
                authority_count: 0,
                additional_count: 0,
            },
            question: Question {
                domain_name: domain_name.clone(),
                q_type: 1,
                q_class: 1,
            },
            answers: Some(vec![ResourceRecord {
                name: domain_name,
                r#type: 1,
                class: 1,
                ttl: 76303,
                data_length: 4,
                data: ResourceData::A(Ipv4Addr::from([93, 184, 216, 34])),
            }]),
        };

        assert_eq!(expected_message.header, parsed_message.header);
        assert_eq!(expected_message.question, parsed_message.question);
        assert_eq!(expected_message.answers, parsed_message.answers);
    }
}
