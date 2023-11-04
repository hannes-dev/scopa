use std::net::UdpSocket;

enum MessageType {
    Query,
    Response,
}

#[derive(Debug)]
struct Query {
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

fn main() -> std::io::Result<()> {
    let client = UdpSocket::bind("127.0.0.1:2000")?;
    let server = UdpSocket::bind("0.0.0.0:2001")?;
    let mut req = [0; 512];
    let mut resp = [0; 512];

    loop {
        let (amt, src) = client.recv_from(&mut req)?;
        server.send_to(&req, "1.1.1.1:53")?;
        let resp_amt = server.recv(&mut resp)?;
        client.send_to(&resp[..resp_amt], src)?;

        println!("Received {} bytes from: {}", amt, src);

        let query = parse_query_header(&req);
        let index = 12;
        let (question, index) = parse_question_section(&req, index);

        println!("{:?}", query);
        println!("{:?}", question);
        print!("Query    ");
        for i in 0..amt {
            print!("{} ", req[i]);
        }
        println!();

        let query = parse_query_header(&resp);
        print!("Response ");
        for i in 0..resp_amt {
            print!("{} ", resp[i]);
        }
        println!();
        println!("{:?}", query);
    }
}

fn parse_query_header(buf: &[u8]) -> Query {
    Query {
        id: ((buf[0] as u16) << 8) | (buf[1] as u16),
        // Query/response bit,
        q_type: (buf[2] >> 3) | 0b00001111,
        // Authorative Answer,
        truncated: flag_set(&buf[2], 0x02),
        recursion_desired: flag_set(&buf[2], 0x01),
        // Recursion available,
        // Zeros
        // Response code
        question_count: ((buf[4] as u16) << 8) | (buf[5] as u16),
        answer_count: ((buf[6] as u16) << 8) | (buf[7] as u16),
        authority_count: ((buf[8] as u16) << 8) | (buf[9] as u16),
        additional_count: ((buf[10] as u16) << 8) | (buf[11] as u16),
    }
}

fn parse_question_section(buf: &[u8], mut index: usize) -> (Question, usize) {
    let mut domain_name = Vec::new();

    let mut length = buf[index] as usize;
    index += 1;

    while length > 0 {
        let label = String::from_utf8_lossy(&buf[index..index + length]).to_string();
        domain_name.push(label);

        index += length;
        length = buf[index] as usize;
        index += 1;
    }

    let q_type = ((buf[index] as u16) << 8) | (buf[index + 1] as u16);
    index += 2;
    let q_class = ((buf[index + 2] as u16) << 8) | (buf[index + 3] as u16);
    index += 2;

    (
        Question {
            domain_name,
            q_type,
            q_class,
        },
        index,
    )
}

fn flag_set(byte: &u8, bit_pos: u8) -> bool {
    byte & bit_pos == bit_pos
}
