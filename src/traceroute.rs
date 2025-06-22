use std::net::{IpAddr, ToSocketAddrs};
use std::time::{Duration, Instant};

use pnet::packet::icmp::{IcmpTypes, echo_request::MutableEchoRequestPacket};
use pnet::packet::{icmp::IcmpPacket, Packet};
use pnet::transport::{icmp_packet_iter, transport_channel, TransportChannelType, TransportProtocol};
use rand::random;

pub async fn run_traceroute(
    host: String,
    max_hops: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let target_addr = (host.as_str(), 0)
        .to_socket_addrs()?
        .find(|a| matches!(a.ip(), IpAddr::V4(_)))
        .expect("No IPv4 address found");

    println!("Traceroute to {} ({})", host, target_addr.ip());

    for ttl in 1..=max_hops {
        let protocol = TransportChannelType::Layer4(TransportProtocol::Ipv4(
            pnet::packet::ip::IpNextHeaderProtocols::Icmp,
        ));
        let (mut tx, mut rx) = transport_channel(1024, protocol)?;
        let mut iter = icmp_packet_iter(&mut rx);
        let identifier = random::<u16>();

        let mut buffer = [0u8; 64];
        let mut packet = MutableEchoRequestPacket::new(&mut buffer).unwrap();

        packet.set_icmp_type(IcmpTypes::EchoRequest);
        packet.set_sequence_number(ttl as u16);
        packet.set_identifier(identifier);
        packet.set_checksum(pnet::packet::icmp::checksum(
            &IcmpPacket::new(packet.packet()).unwrap(),
        ));

        let start = Instant::now();

        // NOTE: On macOS, pnet does not allow setting TTL directly.
        // Real traceroute behavior may require socket2 or raw BSD sockets.

        let _ = tx.send_to(packet, target_addr.ip());

        let deadline = Instant::now() + Duration::from_secs(2);
        let mut reply_ip = None;

        while Instant::now() < deadline {
            if let Ok((packet, addr)) = iter.next() {
                if let Some(_icmp) = IcmpPacket::new(packet.packet()) {
                    reply_ip = Some(addr);
                    break;
                }
            }
        }

        if let Some(ip) = reply_ip {
            let elapsed = start.elapsed();
            println!("{:>2}  {:<15}  {:.2?}", ttl, ip, elapsed);
            if ip == target_addr.ip() {
                println!("Reached destination.");
                break;
            }
        } else {
            println!("{:>2}  *", ttl);
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    Ok(())
