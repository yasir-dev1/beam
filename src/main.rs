use clap::{Parser};
use crate::transfer::send;



#[derive(Parser,Debug)]
#[command(name = "beam" , override_usage = "beam --send <file>\n       beam <code>"
)]
#[command(about = "beam - transfer files between computers")]
#[command(arg_required_else_help(true))]
struct Cli{
    #[arg(short,long,value_name="file")]
    send:Option<String>,
    
    code:Option<String>,
}


mod transfer;



fn main() -> std::result::Result<(), Box<dyn std::error::Error>>{
    let args = Cli::parse();

    if let Some(value) = args.send  {
        println!("Sending {} ...",&value);
        let code = send(&value)?;
        println!("Your code is {}",code);
    }else if let Some(value) = args.code {
        // TODO: RECIVE
        println!("{:?}",value);
    }

    Ok(())
}

