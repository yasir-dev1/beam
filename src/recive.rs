use mdns_sd::{ResolvedService, ServiceDaemon, ServiceEvent};
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::net::TcpStream;


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

fn tcp_connect(addr:&String,code:&String) -> std::result::Result<(),Box<dyn std::error::Error>>{
    println!("Connecting to {}...",addr);
    let mut stream = TcpStream::connect(addr)?;
    stream.write_all(code.as_bytes())?;
    stream.flush()?;
    Ok(())
}

pub fn recive(code:&String) ->std::result::Result<(), Box<dyn std::error::Error>> {
    let services = fetch_codes()?;
    for service in services {
        let service_code  = service.txt_properties.get("code").unwrap().val_str();
        if service_code == code{
            let ip = service.get_addresses_v4();
            let ip = ip.iter().next().unwrap();
            let port = service.port;
            let addr  = format!("{ip}:{port}");
            tcp_connect(&addr, &code)?;
        }
    }

    Ok(())
}
