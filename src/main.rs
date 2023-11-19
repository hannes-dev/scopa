use std::net::UdpSocket;

use scopa::parse_message;

fn main() -> std::io::Result<()> {
    let client = UdpSocket::bind("127.0.0.1:2000")?;
    let server = UdpSocket::bind("0.0.0.0:2001")?;
    let mut request = [0; 512];
    let mut response = [0; 512];

    loop {
        let (request_amt, src) = client.recv_from(&mut request)?;

        server.send_to(&request[..request_amt], "1.1.1.1:53")?;
        let response_amt = server.recv(&mut response)?;

        client.send_to(&response[..response_amt], src)?;

        println!("Received {} bytes from: {}", request_amt, src);
        println!("Sent back a response of {} bytes", response_amt);

        let parsed_request = parse_message(&request);
        let parsed_response = parse_message(&response);

        println!("{parsed_request:?}");
        println!("{parsed_response:?}");
    }
}
