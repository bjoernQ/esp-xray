use std::net::TcpListener;

use esp_xray_server::Message;
use probe_rs::config::{MemoryRegion, TargetSelector};
use probe_rs::rtt::{Rtt, ScanRegion};
use probe_rs::{probe::list::Lister, Permissions};

use clap::Parser;

enum TargetEvent {
    TaskNew = 1,
    TaskExecBegin,
    TaskExecEnd,
    TaskReadyBegin,
    TaskReadyEnd,
    SystemIdle,
}

impl TargetEvent {
    pub fn from(value: u8) -> Self {
        match value {
            1 => Self::TaskNew,
            2 => Self::TaskExecBegin,
            3 => Self::TaskExecEnd,
            4 => Self::TaskReadyBegin,
            5 => Self::TaskReadyEnd,
            6 => Self::SystemIdle,
            _ => {
                panic!("Unknown Event {value}");
            }
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    chip: String,
}

fn normalize(chip_name: &str) -> String {
    chip_name.replace('-', "").to_ascii_lowercase()
}

fn main() {
    let args = Args::parse();
    let chip = normalize(&args.chip);

    let lister = Lister::new();

    let probes = lister.list_all();

    if probes.is_empty() {
        panic!("No debug probes available. Make sure your probe is plugged in, supported and up-to-date.");
    }

    let probe = probes[0].open().unwrap();

    let target_selector = TargetSelector::from(chip);

    let mut session = match probe.attach(target_selector, Permissions::default()) {
        Ok(session) => session,
        Err(err) => {
            panic!("attach failed {:?}", err);
        }
    };

    let memory_map: Vec<MemoryRegion> = vec![session
        .target()
        .memory_map
        .clone()
        .iter()
        .filter(|m| matches!(m, probe_rs::config::MemoryRegion::Ram(_)))
        .next()
        .unwrap()
        .clone()];

    let mut core = match session.core(0) {
        Ok(core) => core,
        Err(err) => {
            panic!("Error attaching to core # 0 {err}");
        }
    };

    eprintln!("Attaching to RTT... {:x?}", &memory_map);

    let mut rtt = match Rtt::attach_region(&mut core,  &ScanRegion::Ram) {
        Ok(rtt) => rtt,
        Err(err) => {
            panic!("Error attaching to RTT: {err}");
        }
    };

    let up_channel = &mut rtt.up_channels()[0];

    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    println!("Attached ... listening on :7878");

    if core.core_halted().unwrap() {
        core.run().unwrap();
    }

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        println!("Connection established!");
        stream
            .set_nonblocking(true)
            .expect("Nonblocking support is required");

        let mut xray = esp_xray_server::SystemViewTarget::new(
            esp_xray_server::TcpTransport::default(),
            stream,
        );

        let mut buf = [0u8; 1024];
        loop {
            if xray.process_incoming() {
                println!("Disconnect requested");
                break;
            }

            let len = up_channel.read(&mut core, &mut buf).unwrap();

            if len != 0 {
                let mut pos = 0;
                while pos < len {
                    let target_event = TargetEvent::from(buf[pos]);
                    match target_event {
                        TargetEvent::TaskNew => {
                            pos += 1;
                            let (index, task) = esp_xray_server::packet::decode_u32(&buf, pos);
                            pos = index;
                            let (index, ts_delta) = esp_xray_server::packet::decode_u32(&buf, pos);
                            pos = index;
                            xray.send(Message::TaskNew(task, ts_delta));
                        }
                        TargetEvent::TaskExecBegin => {
                            pos += 1;
                            let (index, task) = esp_xray_server::packet::decode_u32(&buf, pos);
                            pos = index;
                            let (index, ts_delta) = esp_xray_server::packet::decode_u32(&buf, pos);
                            pos = index;
                            xray.send(Message::TaskExecBegin(task, ts_delta));
                        }
                        TargetEvent::TaskExecEnd => {
                            pos += 1;
                            let (index, ts_delta) = esp_xray_server::packet::decode_u32(&buf, pos);
                            pos = index;
                            xray.send(Message::TaskExecEnd(ts_delta));
                        }
                        TargetEvent::TaskReadyBegin => {
                            pos += 1;
                            let (index, task) = esp_xray_server::packet::decode_u32(&buf, pos);
                            pos = index;
                            let (index, ts_delta) = esp_xray_server::packet::decode_u32(&buf, pos);
                            pos = index;
                            xray.send(Message::TaskReadyBegin(task, ts_delta));
                        }
                        TargetEvent::TaskReadyEnd => {
                            pos += 1;
                            let (index, task) = esp_xray_server::packet::decode_u32(&buf, pos);
                            pos = index;
                            let (index, ts_delta) = esp_xray_server::packet::decode_u32(&buf, pos);
                            pos += index;
                            xray.send(Message::TaskReadyEnd(task, ts_delta));
                        }
                        TargetEvent::SystemIdle => {
                            pos += 1;
                            let (index, ts_delta) = esp_xray_server::packet::decode_u32(&buf, pos);
                            pos = index;
                            xray.send(Message::SystemIdle(ts_delta));
                        }
                    }
                }
            }
        }
    }
}
