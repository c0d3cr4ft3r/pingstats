use std::net::{IpAddr, ToSocketAddrs};
use std::time::{Duration, Instant};

use pnet::packet::icmp::{echo_request::MutableEchoRequestPacket, echo_reply::EchoReplyPacket, IcmpTypes};
use pnet::packet::{icmp::IcmpPacket, Packet};
use pnet::transport::{transport_channel, icmp_packet_iter, TransportChannelType, TransportProtocol};
use rand::random;

pub async fn run_compare(
    hosts: Vec<String>,
    count: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Comparing {} host(s)...\n", hosts.len());

    for host in hosts {
        let addr = match (host.as_str(), 0).to_socket_addrs()?.find(|a| matches!(a.ip(), IpAddr::V4(_))) {
            Some(addr) => addr,
            None => {
                println!("{host:<25} | failed to resolve");
                continue;
            }
        };

        let protocol = TransportChannelType::Layer4(TransportProtocol::Ipv4(
            pnet::packet::ip::IpNextHeaderProtocols::Icmp,
        ));
        let (mut tx, mut rx) = transport_channel(1024, protocol)?;
        let mut iter = icmp_packet_iter(&mut rx);
        let identifier = random::<u16>();

        let mut rtts = Vec::new();
        for seq in 0..count {
            let mut buffer = [0u8; 64];
            let mut packet = MutableEchoRequestPacket::new(&mut buffer).unwrap();

            packet.set_icmp_type(IcmpTypes::EchoRequest);
            packet.set_sequence_number(seq as u16);
            packet.set_identifier(identifier);
            packet.set_checksum(pnet::packet::icmp::checksum(
                &IcmpPacket::new(packet.packet()).unwrap(),
            ));

            let start = Instant::now();
            let _ = tx.send_to(packet, addr.ip());

            let deadline = Instant::now() + Duration::from_secs(1);
            while Instant::now() < deadline {
                if let Ok((packet, _)) = iter.next() {
                    if let Some(echo) = IcmpPacket::new(packet.packet()) {
                        if echo.get_icmp_type() == IcmpTypes::EchoReply {
                            if let Some(reply) = EchoReplyPacket::new(echo.packet()) {
                                if reply.get_identifier() == identifier
                                    && reply.get_sequence_number() == seq as u16
                                {
                                    rtts.push(start.elapsed());
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        if rtts.is_empty() {
            println!("{host:<25} | no reply");
        } else {
            let avg = rtts.iter().map(|d| d.as_secs_f64()).sum::<f64>() / rtts.len() as f64;
            let min = rtts.iter().min().unwrap().as_secs_f64() * 1000.0;
            let max = rtts.iter().max().unwrap().as_secs_f64() * 1000.0;
            let avg_ms = avg * 1000.0;
            let loss = 100.0 * ((count - rtts.len() as u32) as f64) / count as f64;

            println!("{host:<25} | avg: {:>6.2} ms | min: {:>6.2} ms | max: {:>6.2} ms | loss: {:>4.1}%", avg_ms, min, max, loss);
        }
    }

    Ok(())
