mod tui;
mod csv;
mod compare;
mod traceroute;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pingstats", version, about = "A Rust-powered ping tool with TUI and CSV export")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Tui {
        #[arg(short = 'H', long)]
        host: String,

        #[arg(short, long, default_value = "10")]
        count: u32,

        #[arg(short, long, default_value = "1.0")]
        interval: f64,
    },
    Csv {
        #[arg(short = 'H', long)]
        host: String,

        #[arg(short, long, default_value = "50")]
        count: u32,

        #[arg(short, long, default_value = "1.0")]
        interval: f64,

        #[arg(long, default_value = "ping.csv")]
        output: String,
    },
    Compare {
        #[arg(long)]
        hosts: Vec<String>,

        #[arg(short, long, default_value = "3")]
        count: u32,
    },
    Traceroute {
        #[arg(short = 'H', long)]
        host: String,

        #[arg(long, default_value = "30")]
        max_hops: u8,
    },
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Command::Tui { host, count, interval } => {
            tui::run_tui(host, count, interval).await?;
        }

        Command::Csv { host, count, interval, output } => {
            csv::run_csv(host, count, interval, output).await?;
        }
        Command::Compare { hosts, count } => {
            compare::run_compare(hosts, count).await?;
        }
        Command::Traceroute { host, max_hops } => {
            traceroute::run_traceroute(host, max_hops).await?;
        }
    }

    Ok(())
