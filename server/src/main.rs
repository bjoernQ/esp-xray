use std::net::TcpListener;

use esp_xray_server::Message;
use probe_rs::config::TargetSelector;
use probe_rs::rtt::{Rtt, ScanRegion};
use probe_rs::{probe::list::Lister, Permissions};

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

fn main() {
    let lister = Lister::new();

    let probes = lister.list_all();

    if probes.is_empty() {
        panic!("No debug probes available. Make sure your probe is plugged in, supported and up-to-date.");
    }

    let probe = probes[0].open(&lister).unwrap();

    // take from cmd line, or Auto?
    let target_selector = TargetSelector::from("esp32c6");

    let mut session = match probe.attach(target_selector, Permissions::default()) {
        Ok(session) => session,
        Err(err) => {
            panic!("attach failed {:?}", err);
        }
    };

    let memory_map = session.target().memory_map.clone();

    let mut core = match session.core(0) {
        Ok(core) => core,
        Err(err) => {
            panic!("Error attaching to core # 0 {err}");
        }
    };

    eprintln!("Attaching to RTT... {:x?}", &memory_map);

    let mut rtt = match Rtt::attach_region(&mut core, &memory_map, &ScanRegion::Ram) {
        Ok(rtt) => rtt,
        Err(err) => {
            panic!("Error attaching to RTT: {err}");
        }
    };

    let up_channel = rtt.up_channels().take(0).unwrap();

    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    println!("Attached ... listening on :7878");

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
