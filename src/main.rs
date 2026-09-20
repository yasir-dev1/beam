use clap::{Args,Parser};


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


fn main() {
    let args = Cli::parse();

    if let Some(value) = args.send  {
        // TODO: SEND
    }else if let Some(value) = args.code {
        // TODO: RECIVE
    }
}

