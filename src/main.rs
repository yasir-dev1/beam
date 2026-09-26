use clap::{Parser};
use crate::send::send;
use crate::recive::recive;



#[derive(Parser,Debug)]
#[command(name = "beam" , override_usage = "beam --send <file>\n       beam <code>"
)]
#[command(about = "beam - transfer files between computers")]
#[command(arg_required_else_help(true))]
struct Cli{
    #[arg(short,long,value_name="file")]
    send:Option<String>,
   #[arg(short = 'w', long)]
    watch: bool,

    code:Option<String>,
}


mod recive;
mod send;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>>{
    let args = Cli::parse();

    if let Some(value) = args.send  {
        let _ = send(&value,&args.watch)?;
    }else if let Some(value) = args.code {
        let _ = recive(&value);
    }

    Ok(())
}

