use indicatif::ProgressBar;
use mdns_sd::{ResolvedService, ServiceDaemon, ServiceEvent};
use std::fs::File;
use std::io::{self, BufWriter, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::{Arc, Mutex};

fn fetch_codes() -> std::result::Result<Vec<ResolvedService>, Box<dyn std::error::Error>> {
    let mdns = ServiceDaemon::new()?;
    let service_type = "_beam._tcp.local.";
    let receiver = mdns.browse(service_type)?;

    let services = Arc::new(Mutex::new(Vec::<ResolvedService>::new()));

    let services_thread = Arc::clone(&services);

    let handle = std::thread::spawn(move || {
        while let Ok(event) = receiver.recv() {
            match event {
                ServiceEvent::ServiceResolved(resolved) => {
                    let mut services = services_thread.lock().unwrap();
                    services.push(*resolved);
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

fn tcp_connect(
    addr: &String,
    code: &String,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to {}...", addr);
    let mut stream = TcpStream::connect(addr)?;
    stream.write_all(code.as_bytes())?;
    stream.flush()?;

    let mut size_buffer = [0u8; 1024];
    stream.read_exact(&mut size_buffer)?;
    let file_size = std::str::from_utf8(&size_buffer[..])?;
    let file_size = file_size.replace("\0", "");
    let file_size = &file_size.trim();

    let mut name_buffer = [0u8; 1024];
    stream.read_exact(&mut name_buffer)?;
    let file_name = std::str::from_utf8(&name_buffer[..])?;
    let file_name = file_name.replace("\0", "");
    let file_name = file_name.trim();

    println!("Receiving File:{} {} bytes", file_name, file_size);

    let file_path = Path::new(file_name);
    let file = File::create(file_path)?;
    let mut writer = BufWriter::new(file);

    let pb = ProgressBar::new(file_size.parse::<u64>()?);

    let mut tracked_stream = pb.wrap_read(stream);

    io::copy(&mut tracked_stream, &mut writer)?;

    pb.finish_with_message("File received successfully");

    writer.flush()?;

    Ok(())
}

pub fn receive(code: &String) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let services = fetch_codes()?;
    for service in services {
        let service_code = service.txt_properties.get("code").unwrap().val_str();
        if service_code == code {
            let ip = service.get_addresses_v4();
            let ip = ip.iter().next().unwrap();
            let port = service.port;
            let addr = format!("{ip}:{port}");
            tcp_connect(&addr, code)?;
        }
    }

    Ok(())
}
