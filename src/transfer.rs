
use rand::{distr::Alphanumeric,RngExt};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use local_ip_address::local_ip;
use std::fs::File;
use daemonize::Daemonize;

fn gen_code() -> String {
    let mut rng = rand::rng();
    let chars:String = (0..7).map(|_| rng.sample(Alphanumeric) as char).collect();
    return  chars;
}


pub fn send(path: &str) -> std::result::Result<String, Box<dyn std::error::Error>> {
    let code = gen_code();


    let stdout = File::create(format!("/tmp/beam{code}.out")).unwrap();
    let stderr = File::create(format!("/tmp/beam{code}.err")).unwrap();

    let daemonize = Daemonize::new()
        .pid_file(format!("/tmp/beam{code}.pid"))
        .chown_pid_file(true)      
        .working_directory("/tmp") 
        .stdout(stdout)  
        .stderr(stderr);

      
        println!("Your code is {}",code);
        match daemonize.start() {
            Ok(_) => { 
                let host_name = format!("{}.local.", hostname::get()?.to_string_lossy());
                let properties = [("code", code.as_str())];
                let current_ip = local_ip()?;
                
                let daemon = ServiceDaemon::new()?;

                let service = ServiceInfo::new(
                    "_beam._tcp.local.",
                    "Beam",
                    &host_name,
                    current_ip,   
                    53317,
                    &properties[..],
                )?;

                let _ = daemon.register(service)?;

                loop {
                    std::thread::park();
                }
            }
            Err(e) => eprintln!("Error, {}", e),
        }

    println!("{:?}",path);
    Ok(code)
}

