use anyhow::Context;
use anyhow::bail;
use daemonize::Daemonize;
use local_ip_address::local_ip;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use rand::{RngExt, distr::Alphanumeric};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::{IpAddr, TcpListener, TcpStream};
use std::os::unix::{ffi::OsStrExt, fs::MetadataExt};
use std::path::{Path, PathBuf};
use anyhow::Result;

fn gen_code() -> String {
    let mut rng = rand::rng();
    let chars: String = (0..7).map(|_| rng.sample(Alphanumeric) as char).collect();
    chars
}

fn handle_client(mut stream: TcpStream) -> Result<()> {
    let mut buffer = [0u8; 1024];
    stream.read_exact(&mut buffer).context("failed to read request from client")?;
    println!("Waiting for code");
    let request = std::str::from_utf8(&buffer[..]).context("received invalid UTF-8 for beam code")?;
    let request = request.replace("\0", "");
    let request = request.trim();
    let path = get_file_path(request).context("failed to find matching file with code")?;
    let path = Path::new(&path);

    let metadata = path.metadata()
    .with_context(|| format!("failed to inspect file {}", path.display()))?;

    if !metadata.is_file() {
        let message = format!(
            "given path is not a file {}",
            path.display()
        );
        stream.write_all(message.as_bytes()).context("failed to send message to client")?;
        bail!(format!("given path is not a file {}",path.display()))
    }

    let file = File::open(&path).with_context(|| format!("failed to open file {}",path.display()))?;
    let file_size = file.metadata().context("failed get file metadata")?.size();
    let mut reader = BufReader::new(file);
    let file_name = match path.file_name() {
        Some(name) => name,
        None => bail!("failed to get file name from path `{}`", path.display())
    };

    stream.write_all(file_size.to_string().as_bytes()).context("failed send file size to client")?;
    stream.write_all(file_name.as_bytes()).context("failed send file name to client")?;

    let mut buffer = [0u8; 4096];
    let mut writer = BufWriter::new(stream);

    loop {
        let bytes_read = reader.read(&mut buffer).with_context(|| format!("failed read file {}",path.display()))?;
        if bytes_read == 0 {
            break;
        }
        writer.write_all(&buffer[..bytes_read]).context("failed send file content to client")?;
    }

    writer.flush().context("failed to flush file")?;

    Ok(())
}

fn register_service(host_name: &str,current_ip: &IpAddr,code: &String,) -> Result<String> {
    let properties = [("code", code.as_str())];
    let daemon = ServiceDaemon::new().context("failed create mDNS service deamon")?;

    let service = ServiceInfo::new(
        "_beam._tcp.local.",
        &format!("Beam_{}", code),
        host_name,
        current_ip,
        53317,
        &properties[..],
    ).context("failed create mDNS service")?;

    daemon.register(service.clone()).context("failed register mDNS service")?;

    Ok(service.get_fullname().to_string())
}

fn register_file(path: &String,code: &String,service_name: &String) -> Result<()> {
    let mut dir = match dirs::data_local_dir(){
        Some(dir) => dir,
        None => bail!("failed to found local dir"),
    };
    dir.push("beam");

    std::fs::create_dir_all(&dir).with_context(|| format!("failed to create {}",dir.display()))?;

    let file_path = dir.join("beam.json");

    let data = serde_json::json!({
        "code": code,
        "path": path,
        "service_name":service_name,
    });

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path).with_context(|| format!("failed to open {}",file_path.display()))?;

    let json = serde_json::to_string(&data)
        .context("failed to serialize beam file entry")?;

    writeln!(file, "{}", json)
        .with_context(|| format!("failed to write {}", file_path.display()))?;

    Ok(())
}

fn get_file_path(code: &str) -> Result<String> {
    let mut dir = match dirs::data_local_dir(){
        Some(dir) => dir,
        None => bail!("failed to found local dir"),
    };
    dir.push("beam");

    let file_path = dir.join("beam.json");
    let file = File::open(&file_path).with_context(|| format!("failed open file {}",file_path.display()))?;
    let reader = BufReader::new(file);
    for (index , line) in reader.lines().enumerate() {
        let line = line
          .with_context(|| format!("failed to read line {} from {}",index + 1,file_path.display()))?;
        
        let data: serde_json::Value = serde_json::from_str(&line)
        .context("failed to extract file data from json")?;

        let path = match data["path"].as_str(){
            Some(path) => path,
            None => bail!("failed to parse file path")
        };

        let current_code = match data["code"].as_str(){
            Some(code) => code,
            None => bail!("failed to parse code")
        };

        if current_code == code {
            return Ok(String::from(path));
        }
    }

    bail!("no file found for code `{code}`");

}

fn check_already_sent(path: &String) -> Result<bool> {
    let mut dir = match dirs::data_local_dir(){
        Some(dir) => dir,
        None => bail!("failed to found local dir"),
    };
    dir.push("beam");

    let file_path = dir.join("beam.json");
    
    if !file_path.exists(){return Ok(false);}

    let file = File::open(&file_path).with_context(|| format!("failed open file {}",file_path.display()))?;

    let reader = BufReader::new(file);

    for (index,line) in reader.lines().enumerate() {
        let line = line
        .with_context(|| format!("failed to read line {} from {}",index + 1,file_path.display()))?;
        
        let data: serde_json::Value = serde_json::from_str(&line)
        .context("failed to extract file data from json")?;
        
        let current_path = match data["path"].as_str(){
            Some(path) => path,
            None => bail!("failed to parse path")
        };

        if current_path == path {
            return Ok(true);
        }
    }

    Ok(false)
}

fn send_file(code: &String, path: &String) -> Result<()> {
    let host_name = format!("{}.local.", hostname::get().context("failed to get hostname")?.to_string_lossy());
    let current_ip = local_ip().context("failed to get local ip address")?;
    let ip_str = current_ip.to_string();
    let addr = format!("{ip_str}:53317");

    let service_name = register_service(&host_name, &current_ip, code).with_context(|| "failed to register service")?;
    register_file(path, code, &service_name)?;

    let listener = TcpListener::bind(&addr).with_context(|| format!("failed to bind tcp listener at {} address",&addr))?;
    println!("TCP Listening addr: {}", addr);
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(|| {
                    if let Err(err) = handle_client(stream) {
                        eprintln!("client handling failed: {err:#}");
                    }
                });

            }
           Err(e) => return Err(e).context("failed to accept TCP connection"),

        }
    }
    Ok(())
}

fn get_full_path(filename: &str) -> Result<PathBuf> {
    let current_dir = env::current_dir()
    .with_context(|| "failed to get current dir")?;

    let path = current_dir.join(filename);

    if !path.exists() {
        bail!("File not found: {}", path.display());
    }

    Ok(path)

}

pub fn send(path: &str,watch: &bool) -> Result<()> {
    let code = gen_code();
    let path = get_full_path(&path)?;
    let path = path.display();
    if check_already_sent(&path.to_string())? {
        bail!("{} Is already sent",path);
    }
    println!("Your BeamCode is :{code} ");

    if *watch {
        send_file(&code, &path.to_string()).with_context(|| format!("failed sending {}",path))?;
    } else {
        let stdout = File::create(format!("/tmp/beam{code}.out")).with_context(|| format!("failed create {}",format!("/tmp/beam{code}.out")))?;
        let stderr = File::create(format!("/tmp/beam{code}.err")).with_context(|| format!("failed create {}",format!("/tmp/beam{code}.err")))?;

        let daemonize = Daemonize::new()
            .pid_file(format!("/tmp/beam{code}.pid"))
            .chown_pid_file(true)
            .working_directory("/tmp")
            .stdout(stdout)
            .stderr(stderr);

        daemonize.start().with_context(|| "failed to start daemon")?;

        send_file(&code, &path.to_string()).with_context(|| format!("failed sending {}",path))?;
    }

    Ok(())
}

pub fn cancel(code: &str) -> Result<()> {
    let pid_path = format!("/tmp/beam{code}.pid");
    let pid = std::fs::read_to_string(&pid_path).with_context(|| format!("failed to read {}",pid_path))?.trim().to_string();

    let status = std::process::Command::new("kill")
        .args(["-TERM", &pid])
        .status()
        .with_context(|| format!("failed to execute kill for pid {}", pid))?;

    if !status.success() {
        bail!("failed to terminate service with pid {}", pid);
    }

    let mut dir = match dirs::data_local_dir(){
        Some(dir) => dir,
        None => bail!("failed to found local dir"),
    };
    dir.push("beam");

    let file_path = dir.join("beam.json");
    let mut lines = Vec::new();

    let binding = std::fs::read_to_string(&file_path)
        .with_context(|| format!("failed to read {}", file_path.display()))?;
    
    let readed_lines = binding.lines();

    for line in readed_lines
    {
        let value: serde_json::Value = serde_json::from_str(line)
            .context("failed to parse JSON entry while cancelling")?;

        if value["code"].as_str() != Some(code) {
            lines.push(line);
        }
    }


    std::fs::write(&file_path, &lines.join("\n"))
    .with_context(|| format!("failed to write {}", file_path.display()))?;

    Ok(())
}
