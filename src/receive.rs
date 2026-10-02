use indicatif::ProgressBar;
use mdns_sd::{ResolvedService, ServiceDaemon, ServiceEvent};
use std::fs::File;
use std::io::{self, BufWriter, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use anyhow::{Context,Result, anyhow, bail};

fn fetch_codes() -> Result<Vec<ResolvedService>> {
    let mdns = ServiceDaemon::new().context("Failed start mDNS deamon.")?;
    let receiver = mdns.browse("_beam._tcp.local.").context("Failed to browse for ")?;

    let handle = std::thread::spawn(move || {
        let mut services = Vec::new();
        while let Ok(event) = receiver.recv() {
            if let ServiceEvent::ServiceResolved(resolved) = event{
                services.push(*resolved);
            }
        }
        services
    });

    
    std::thread::sleep(std::time::Duration::from_secs(1));
    mdns.shutdown().context("Failed to shutdown mDNS deamon")?;
    let services =    handle.join().map_err(|_| anyhow!("service reciver thread panciked"))?;

    Ok(services)
}

fn tcp_connect(addr: &String, code: &String) -> Result<()> {
    println!("Connecting to {}...", addr);
    let mut stream = TcpStream::connect(addr).context("Connection failed")?;
    stream.write_all(code.as_bytes()).context("Failed sending code")?;
    stream.flush().context("Failed to flush code to sender")?;

    let mut size_buffer = [0u8; 1024];
    stream.read_exact(&mut size_buffer).context("Failed to read file size from sender device")?;
    let file_size = std::str::from_utf8(&size_buffer[..]).context("received invalid UTF-8")?;
    let file_size = file_size.replace("\0", "");
    let file_size = &file_size.trim();

    let mut name_buffer = [0u8; 1024];
    stream.read_exact(&mut name_buffer).context("Failed to read file name from sender device")?;
    let file_name = std::str::from_utf8(&name_buffer[..]).context("received invalid UTF-8")?;
    let file_name = file_name.replace("\0", "");
    let file_name = file_name.trim();

    println!("Receiving File:{} {} bytes", file_name, file_size);

    let file_path = Path::new(file_name);
    let file = File::create(file_path).context("Failed to create output file")?;
    let mut writer = BufWriter::new(file);

    let file_size = file_size.parse::<u64>().context("received invalid file size")?;
    
    let pb = ProgressBar::new(file_size);

    let mut tracked_stream = pb.wrap_read(stream);

    io::copy(&mut tracked_stream, &mut writer).context("Failed Receive file content")?;

    pb.finish_with_message("File received successfully");

    writer.flush().context("failed to flush output file")?;

    Ok(())
}

pub fn receive(code: &String) -> Result<()> {
    let services = fetch_codes().context("Failed to fetch available files")?;
    for service in services {
        let service_code = match service.txt_properties.get("code"){
            Some(code) => code.val_str(),
            None => continue,
        };
        if service_code == code {
            let ip = service.get_addresses_v4();
            let ip = match ip.iter().next() {
                Some(ip) => ip ,
                None => bail!("failed to get IPv4 address of sender"),
            };
            let port = service.port;
            let addr = format!("{ip}:{port}");
            tcp_connect(&addr, code).context("failed to receive file from sender")?;
            return  Ok(());
        }
    }
    bail!("no sender found for code `{code}`");
}
