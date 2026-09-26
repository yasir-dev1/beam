
use rand::{distr::Alphanumeric,RngExt};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use local_ip_address::local_ip;
use std::{fs::File, io::{BufRead, BufReader, Read, Write}, net::{IpAddr, TcpListener, TcpStream}};
use daemonize::Daemonize;
use serde_json::json;
use std::fs::OpenOptions;



fn gen_code() -> String {
    let mut rng = rand::rng();
    let chars:String = (0..7).map(|_| rng.sample(Alphanumeric) as char).collect();
    return  chars;
}

fn handle_clinet(mut stream:TcpStream)-> std::result::Result<(),Box<dyn  std::error::Error + Send + Sync>>{
    let mut buffer = [0;1024];
    stream.read(&mut buffer).expect("Error Reading");
    let request = String::from_utf8_lossy(&buffer[..]);
    println!("Request : {request:?}");
    let response = get_file_path(&request)?;
    stream.write(&response.as_bytes())?;
    Ok(())
}

fn register_service(host_name:&String,current_ip:&IpAddr,code:&String) -> std::result::Result<(),Box<dyn  std::error::Error + Send + Sync>> {
    let properties = [("code", code.as_str())];
    let daemon = ServiceDaemon::new()?;

    let service = ServiceInfo::new(
        "_beam._tcp.local.",
        "Beam",
        host_name,
        current_ip,   
        53317,
        &properties[..],
    )?;

    let _ = daemon.register(service)?;
    
    Ok(())
}

fn register_file(path:&String,code:&String)-> std::result::Result<(),Box<dyn  std::error::Error + Send + Sync>> {

    let mut dir = dirs::data_local_dir().unwrap();
    dir.push("beam");

    std::fs::create_dir_all(&dir)?;

    let file_path = dir.join("beam.json");

    let data = json!({
        "code": code,
        "path": path,
    });

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)?;

    let json = serde_json::to_string(&data)?;
    writeln!(file, "{}", json)?;

    Ok(())


}

fn get_file_path(code:&str)-> std::result::Result<String,Box<dyn std::error::Error + Send + Sync>> {
    let mut dir = dirs::data_local_dir().unwrap();
    dir.push("beam");

    let file_path = dir.join("beam.json");
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;

        let data:serde_json::Value = serde_json::from_str(&line)?;

        if data["code"].as_str().unwrap() == code{
            return Ok(String::from(data["path"].as_str().unwrap()));
        }   
    }

    Ok(String::from(""))
}

fn check_already_sent(path:&String) -> std::result::Result<bool,Box<dyn  std::error::Error + Send + Sync>>{
   let mut dir = dirs::data_local_dir().unwrap();
    dir.push("beam");

    let file_path = dir.join("beam.json");

    let file = match File::open(file_path) {
        Ok(file) => file,
        Err(_) => return Ok(false),
    };

    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;

        let data: serde_json::Value = serde_json::from_str(&line)?;

        if data["path"].as_str().unwrap() == path {
            return Ok(true);
        }
    }

    Ok(false)
}

fn send_file(code:&String,path: &String)->std::result::Result<(), Box<dyn std::error::Error + Send + Sync>>{
    let host_name = format!("{}.local.", hostname::get()?.to_string_lossy());
    let current_ip = local_ip()?;
    let ip_str = current_ip.to_string();
    let addr = format!("{ip_str}:53317");

    register_file(path, code)?;
    register_service(&host_name, &current_ip, &code)?;  
    
    let listener = TcpListener::bind(&addr)?;
    println!("TCP Listning addr: {}",&addr);
    for stream in listener.incoming(){
        match stream {
            Ok(stream) => {
                std::thread::spawn(|| handle_clinet(stream) );
            }
            Err(e) => {
                eprintln!("Error {}",e);
            }
        }
    }
    Ok(())
}
             
pub fn send(path: &str,watch:&bool) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let code = gen_code();
    if check_already_sent(&path.to_string())? {
        println!("This File is already sent");
        return Ok(());
    }
    
    if *watch {
        send_file(&code,&path.to_string())?;
    }else{
        let stdout = File::create(format!("/tmp/beam{code}.out")).unwrap();
        let stderr = File::create(format!("/tmp/beam{code}.err")).unwrap();

        let daemonize = Daemonize::new()
            .pid_file(format!("/tmp/beam{code}.pid"))
            .chown_pid_file(true)      
            .working_directory("/tmp") 
            .stdout(stdout)  
            .stderr(stderr);

        match daemonize.start() {
            Ok(_) => send_file(&code,&path.to_string())?,
            Err(e) => eprintln!("Error, {}", e),
        }

    }

    Ok(())
}
