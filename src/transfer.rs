
use rand::{distr::Alphanumeric,RngExt};
use mdns_sd::{ResolvedService, ServiceDaemon, ServiceEvent, ServiceInfo};
use local_ip_address::local_ip;
use std::{fs::File, io::{Read, Write}, net::{IpAddr, TcpListener, TcpStream}, sync::{Arc, Mutex}};
use daemonize::Daemonize;


fn gen_code() -> String {
    let mut rng = rand::rng();
    let chars:String = (0..7).map(|_| rng.sample(Alphanumeric) as char).collect();
    return  chars;
}

fn handle_clinet(mut stream:TcpStream){
    let mut buffer = [0;1024];
    stream.read(&mut buffer).expect("Error Reading");
    let request = String::from_utf8_lossy(&buffer[..]);
    println!("Request : {request:?}");
    let response = "Hello\n".as_bytes();
    stream.write(&response).expect("Error Sending Response");
}

fn register_service(host_name:&String,current_ip:&IpAddr,code:&String) -> std::result::Result<(),Box<dyn  std::error::Error>> {
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
                let current_ip = local_ip()?;
                let ip_str = current_ip.to_string();
                let addr = format!("{ip_str}:53317");
                
                register_service(&host_name, &current_ip, &code)?;    
                
                let listener = TcpListener::bind(&addr)?;
                println!("TCP Listning addr: {}",&addr);
                for stream in listener.incoming(){
                    match stream {
                        Ok(stream) => {
                            println!("There are incoming");
                            std::thread::spawn(|| handle_clinet(stream) );
                        }
                        Err(e) => {
                            eprintln!("Error {}",e)
                        }
                    }
        }
            }
            Err(e) => eprintln!("Error, {}", e),
        }

    println!("{:?}",path);
    Ok(code)
}


pub fn recive(code:&str) ->std::result::Result<(), Box<dyn std::error::Error>> {
    let services = fetch_codes()?;
    for service in services {
        let service_code  = service.txt_properties.get("code").unwrap().val_str();
        if service_code == code{
            let ip = service.get_addresses_v4();
            let ip = ip.iter().next().unwrap();
            let port = service.port;
            println!("ip : {:?} , port: {port:?}",ip);
        }
    }

    Ok(())
}

fn fetch_codes() -> std::result::Result<Vec<Box<ResolvedService>>, Box<dyn std::error::Error>> {
    let mdns = ServiceDaemon::new()?;
    let service_type = "_beam._tcp.local.";
    let receiver =  mdns.browse(service_type)?;
    
    let  services = Arc::new(Mutex::new(Vec::<Box<ResolvedService>>::new()));

    let services_thread  = Arc::clone(&services);
    
    let handle = std::thread::spawn(move || {
        while let Ok(event) = receiver.recv() {
            match event {
                ServiceEvent::ServiceResolved(resolved) => {
                    let mut services = services_thread .lock().unwrap();
                    services.push(resolved);
                }
                other_event => {
                    let _ = other_event;
                }
            }
        }
    });
    
 

    std::thread::sleep(std::time::Duration::from_secs(1));
    mdns.shutdown().unwrap();
    handle.join().unwrap();

    let services = Arc::try_unwrap(services).unwrap().into_inner().unwrap();
    Ok(services)
}