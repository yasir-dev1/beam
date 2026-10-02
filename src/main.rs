mod receive;
mod send;
use crate::receive::receive;
use crate::send::{cancel, send};
use clap::Parser;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Parser, Debug)]
#[command(
    name = "beam",
    override_usage = "beam --send <file>\n       beam <code>"
)]
#[command(about = "beam - transfer files between computers")]
#[command(arg_required_else_help(true))]
struct Cli {
    #[arg(short, long, value_name = "file")]
    send: Option<String>,
    #[arg(short = 'w', long)]
    watch: bool,
    #[arg(short, long)]
    list: bool,
    #[arg(short, long, value_name = "code")]
    cancel: Option<String>,

    code: Option<String>,
}

fn list_all() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut dir = dirs::data_local_dir().unwrap();
    dir.push("beam");

    let file_path = dir.join("beam.json");

    let file = match File::open(file_path) {
        Ok(file) => file,
        Err(_) => return Ok(()),
    };

    let reader = BufReader::new(file);
    println!("{:<50} {:<10} Service Name", "Path", "Code");
    println!("{:-<50} {:-<10} {:-<20}", "", "", "");

    for line in reader.lines() {
        let line = line?;
        let data: serde_json::Value = serde_json::from_str(&line)?;

        println!(
            "{:<50} {:<10} {}",
            data["path"].as_str().unwrap(),
            data["code"].as_str().unwrap(),
            data["service_name"].as_str().unwrap(),
        );
    }

    Ok(())
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Cli::parse();

    if args.list {
        let _ = list_all();
    } else if let Some(value) = args.send {
        send(&value, &args.watch)?;
    } else if let Some(value) = args.code {
        let _ = receive(&value);
    } else if let Some(value) = args.cancel {
        let _ = cancel(&value);
    }

    Ok(())
}
