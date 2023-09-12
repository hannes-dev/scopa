use std::net::UdpSocket;
enum MessageType {
    Query,
    Response,
}

#[derive(Debug)]
enum QueryType {
    Normal,
    Inverse,
    Status,
}

#[derive(Debug)]
struct Query {
    id: u16,
    q_type: QueryType,
    truncated: bool,
    recursion_desired: bool,

    question_count: u16,
    answer_count: u16,
    authority_count:u16,
    additional_count:u16,
}
fn main() -> std::io::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:34254")?;
    let mut buf = [0; 100];

    loop {
        let (amt, src) = socket.recv_from(&mut buf)?;
    
        println!("Received {} bytes from: {}", amt, src);
        
        let query = parse_query_header(&buf);

        println!("{:?}", query);
        for i in 12..amt {
            print!("{} ", buf[i]);
        }
        println!("");
    }

    Ok(())
}

fn flag_set(byte: &u8, bit_pos:u8) -> bool {
    byte & bit_pos == bit_pos
}

fn parse_query_header(buf: &[u8]) -> Query {
    Query {
        id: ((buf[0] as u16) << 8) | (buf[1] as u16),
        q_type: QueryType::Normal,
        truncated: flag_set(&buf[2], 0x02),
        recursion_desired: flag_set(&buf[2], 0x01),
        question_count: ((buf[4] as u16) << 8) | (buf[5] as u16),
        answer_count: ((buf[6] as u16) << 8) | (buf[7] as u16),
        authority_count: ((buf[8] as u16) << 8) | (buf[9] as u16),
        additional_count: ((buf[10] as u16) << 8) | (buf[11] as u16),
    }
}

// let op_code:u8 = buf[2] >> 3 & 0x0F;
