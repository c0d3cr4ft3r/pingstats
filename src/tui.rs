use crossterm::{
    event::{poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::random;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    symbols,
    text::Span,
    widgets::{Axis, Block, Borders, Chart, Dataset, Paragraph},
    Terminal,
};
use std::{
    collections::VecDeque,
    io::{self, stdout},
    net::{IpAddr, ToSocketAddrs},
    time::{Duration, Instant},
};
use tokio::time::sleep;

use pnet::packet::icmp::{echo_reply::EchoReplyPacket, echo_request::MutableEchoRequestPacket, IcmpTypes};
use pnet::packet::{icmp::IcmpPacket, Packet};
use pnet::transport::{icmp_packet_iter, transport_channel, TransportChannelType, TransportProtocol};

pub async fn run_tui(host: String, count: u32, interval: f64) -> Result<(), Box<dyn std::error::Error>> {
    let addr = (host.as_str(), 0)
        .to_socket_addrs()?
        .find(|a| matches!(a.ip(), IpAddr::V4(_)))
        .expect("No IPv4 address found");

    let protocol = TransportChannelType::Layer4(TransportProtocol::Ipv4(
        pnet::packet::ip::IpNextHeaderProtocols::Icmp,
    ));
    let (mut tx, mut rx) = transport_channel(1024, protocol)?;
    let mut iter = icmp_packet_iter(&mut rx);

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut rtts = Vec::new();
    let mut recent_rtts = VecDeque::with_capacity(50);
    let mut received_count = 0;
    let identifier = random::<u16>();

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
        let mut received = false;

        while Instant::now() < deadline {
            if let Ok((packet, _)) = iter.next() {
                if let Some(echo) = IcmpPacket::new(packet.packet()) {
                    if echo.get_icmp_type() == IcmpTypes::EchoReply {
                        if let Some(reply) = EchoReplyPacket::new(echo.packet()) {
                            if reply.get_identifier() == identifier && reply.get_sequence_number() == seq as u16 {
                                let elapsed = start.elapsed();
                                rtts.push(elapsed);
                                recent_rtts.push_back(elapsed);
                                if recent_rtts.len() > 50 {
                                    recent_rtts.pop_front();
                                }
                                received_count += 1;
                                received = true;
                                break;
                            }
                        }
                    }
                }
            }
        }

        if !received {
            recent_rtts.push_back(Duration::from_millis(0));
            if recent_rtts.len() > 50 {
                recent_rtts.pop_front();
            }
        }

        draw_ui(&mut terminal, &recent_rtts, &rtts, &host, count, received_count)?;

        sleep(Duration::from_secs_f64(interval)).await;

        if poll(Duration::from_millis(1))? {
            if let Event::Key(key) = read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    Ok(())
}

fn draw_ui<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    recent_rtts: &VecDeque<Duration>,
    all_rtts: &Vec<Duration>,
    host: &str,
    total_count: u32,
    received_count: u32,
) -> io::Result<()> {
    terminal.draw(|f| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
            .split(f.size());

        let points: Vec<(f64, f64)> = recent_rtts
            .iter()
            .enumerate()
            .map(|(i, d)| (i as f64, d.as_secs_f64() * 1000.0))
            .collect();

        let dataset = Dataset::default()
            .name("RTT (ms)")
            .marker(symbols::Marker::Dot)
            .style(Style::default().fg(Color::Cyan))
            .data(&points);

        let chart = Chart::new(vec![dataset])
            .block(Block::default().borders(Borders::ALL).title("Latency (ms)"))
            .x_axis(
                Axis::default()
                    .title("ping #")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, 50.0]),
            )
            .y_axis(
                Axis::default()
                    .title("ms")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, 100.0]),
            );

        f.render_widget(chart, chunks[0]);

        let stats = if all_rtts.is_empty() {
            "No data yet".to_string()
        } else {
            let min = all_rtts.iter().min().unwrap();
            let max = all_rtts.iter().max().unwrap();
            let avg = all_rtts.iter().map(|d| d.as_secs_f64()).sum::<f64>() / all_rtts.len() as f64;
            let loss = 100.0 * ((total_count - received_count) as f64) / total_count as f64;

            format!(
                "host: {}\ntransmitted: {}\nreceived: {}\nloss: {:.1}%\nmin: {:.2?}  avg: {:.2?}  max: {:.2?}\n(press 'q' to quit)",
                host, total_count, received_count, loss,
                min, Duration::from_secs_f64(avg), max
            )
        };

        let stats_block = Paragraph::new(Span::raw(stats))
            .block(Block::default().borders(Borders::ALL).title("Stats"));

        f.render_widget(stats_block, chunks[1]);
    }).map(|_| ())
}
