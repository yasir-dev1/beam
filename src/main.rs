mod receive;
mod send;
use crate::receive::receive;
use crate::send::{cancel, send};
use anyhow::{Context, Result,bail};
use clap::Parser;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Parser, Debug)]
#[command(name = "beam")]
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

fn list_all() -> Result<()> {
    let mut dir = match dirs::data_local_dir(){
        Some(dir) => dir,
        None => bail!("failed to found local dir"),
    };
    dir.push("beam");

    let file_path = dir.join("beam.json");

    let file = File::open(&file_path).with_context(|| format!("failed to open {}",file_path.display()))?;

    let reader = BufReader::new(file);
    println!("{:<50} {:<10} Service Name", "Path", "Code");
    println!("{:-<50} {:-<10} {:-<20}", "", "", "");

    for (index,line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("failed to read line {} from {}",index + 1,file_path.display()))?;
        let data: serde_json::Value = serde_json::from_str(&line).context("failed to extract file data from json")?;

        let path = match data["path"].as_str(){
            Some(path) => path,
            None => bail!("failed to parse file path")
        };

        let code = match data["code"].as_str(){
            Some(code) => code,
            None => bail!("failed to parse code")
        };

        
        let service_name = match data["service_name"].as_str(){
            Some(service_name) => service_name,
            None => bail!("failed to parse service name")
        };

        println!(
            "{:<50} {:<10} {}",
            path,
            code,
            service_name,
        );
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = Cli::parse();

    if args.list {
        list_all()?;
    } else if let Some(value) = args.send {
        send(&value, &args.watch)?;
    } else if let Some(value) = args.code {
        receive(&value)?;
    } else if let Some(value) = args.cancel {
        cancel(&value)?;
    }

    Ok(())
}
