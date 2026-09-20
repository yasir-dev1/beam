use rand::{distr::Alphanumeric,RngExt};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use local_ip_address::local_ip;

fn gen_code() -> String {
    let mut rng = rand::rng();
    let chars:String = (0..7).map(|_| rng.sample(Alphanumeric) as char).collect();
    return  chars;
}


pub fn send(path: &str) -> std::result::Result<String, Box<dyn std::error::Error>> {
    let code = gen_code();
    let properties = [("code", &code)];
    let local_ip = local_ip();
    let daemon = ServiceDaemon::new()?;

    if let Ok(local_ip) = local_ip {
        let service = ServiceInfo::new(
            "_beam._tcp.local.",
            "Beam",
            "beam_device.local.",
            local_ip,
            53317,
            &properties[..],
        )?;

        let _ = daemon.register(service)?;

    println!("Service registered successfully: IP {}, Port {}", local_ip, 53317);
    std::thread::park();

    } else {
        return Err("Failed to determine local IP address".into());
    }
    println!("{:?}",path);
    Ok(code)
}

