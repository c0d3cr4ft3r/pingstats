use std::fs::File;
use std::io::{BufWriter, Write};
use std::net::{IpAddr, ToSocketAddrs};
use std::time::{Duration, Instant};

use tokio::time::sleep;

use rand::random;

use pnet::packet::icmp::{echo_reply::EchoReplyPacket, echo_request::MutableEchoRequestPacket, IcmpTypes};
use pnet::packet::{icmp::IcmpPacket, Packet};
use pnet::transport::{icmp_packet_iter, transport_channel, TransportChannelType, TransportProtocol};

pub async fn run_csv(
    host: String,
    count: u32,
    interval: f64,
    output: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = (host.as_str(), 0)
        .to_socket_addrs()?
        .find(|a| matches!(a.ip(), IpAddr::V4(_)))
        .expect("No IPv4 address found");

    let protocol = TransportChannelType::Layer4(TransportProtocol::Ipv4(
        pnet::packet::ip::IpNextHeaderProtocols::Icmp,
    ));
    let (mut tx, mut rx) = transport_channel(1024, protocol)?;
    let mut iter = icmp_packet_iter(&mut rx);

    let path = std::path::Path::new(&output);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    let mut writer = BufWriter::new(File::create(path)?);


    writeln!(writer, "seq,timestamp,rtt_ms")?;

    let identifier = random::<u16>();
    let mut received = 0;

    for seq in 0..count {
        let mut buffer = [0u8; 64];
        let mut packet = MutableEchoRequestPacket::new(&mut buffer).unwrap();

        packet.set_icmp_type(IcmpTypes::EchoRequest);
        packet.set_sequence_number(seq as u16);
        packet.set_identifier(identifier);
        packet.set_checksum(pnet::packet::icmp::checksum(&IcmpPacket::new(packet.packet()).unwrap()));

        let start = Instant::now();
        let _ = tx.send_to(packet, addr.ip());

        let timeout = Duration::from_secs(1);
        let deadline = Instant::now() + timeout;
        let mut rtt = None;

        while Instant::now() < deadline {
            if let Ok((packet, _)) = iter.next() {
                if let Some(echo) = IcmpPacket::new(packet.packet()) {
                    if echo.get_icmp_type() == IcmpTypes::EchoReply {
                        if let Some(reply) = EchoReplyPacket::new(echo.packet()) {
                            if reply.get_identifier() == identifier && reply.get_sequence_number() == seq as u16 {
                                rtt = Some(start.elapsed());
                                received += 1;
                                break;
                            }
                        }
                    }
                }
            }
        }

        let timestamp = chrono::Utc::now().to_rfc3339();
        let rtt_ms = rtt.map(|d| d.as_secs_f64() * 1000.0).unwrap_or(0.0);
        writeln!(writer, "{},{},{:.3}", seq, timestamp, rtt_ms)?;

        sleep(Duration::from_secs_f64(interval)).await;
    }

    writer.flush()?;

    let loss = 100.0 * ((count - received) as f64) / count as f64;
    println!(
        "\nCSV written to: {}\n{} packets sent, {} received, {:.1}% packet loss",
        output, count, received, loss
    );

    Ok(())
}
