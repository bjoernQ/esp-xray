use std::net::TcpListener;

use esp_xray_server::Message;
use probe_rs::rtt::{Channels, Rtt, RttChannel, ScanRegion};
use probe_rs::{config::TargetSelector, probe::DebugProbeInfo};
use probe_rs::{probe::list::Lister, Permissions};

fn main() {
    let lister = Lister::new();

    let probes = lister.list_all();

    if probes.is_empty() {
        panic!("No debug probes available. Make sure your probe is plugged in, supported and up-to-date.");
    }

    let probe = probes[0].open(&lister).unwrap();

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

    let mut rtt = match Rtt::attach_region(
        &mut core,
        &memory_map,
        &ScanRegion::Range(0x40800000..0x408020d0),
    ) {
        Ok(rtt) => rtt,
        Err(err) => {
            panic!("Error attaching to RTT: {err}");
        }
    };

    let up_channel = rtt.up_channels().take(0).unwrap();

    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        println!("Connection established!");

        let mut xray = esp_xray_server::SystemViewTarget::new(
            esp_xray_server::TcpTransport::default(),
            stream,
        );

        loop {
            let mut buf = [0u8; 1024];
            let len = up_channel.read(&mut core, &mut buf).unwrap();

            if len != 0 {
                let mut pos = 0;
                while pos < len {
                    match buf[pos] {
                        1 => {
                            pos += 1;
                            let (index, task) = esp_xray_server::packet::decode_u32(&buf, pos);
                            pos = index;
                            let (index, ts_delta) = esp_xray_server::packet::decode_u32(&buf, pos);
                            pos = index;
                            xray.send(Message::TaskNew(task, ts_delta));
                        }
                        2 => {
                            pos += 1;
                            let (index, task) = esp_xray_server::packet::decode_u32(&buf, pos);
                            pos = index;
                            let (index, ts_delta) = esp_xray_server::packet::decode_u32(&buf, pos);
                            pos = index;
                            xray.send(Message::TaskExecBegin(task, ts_delta));
                        }
                        3 => {
                            pos += 1;
                            let (index, ts_delta) = esp_xray_server::packet::decode_u32(&buf, pos);
                            pos = index;
                            xray.send(Message::TaskExecEnd(ts_delta));
                        }
                        4 => {
                            pos += 1;
                            let (index, task) = esp_xray_server::packet::decode_u32(&buf, pos);
                            pos = index;
                            let (index, ts_delta) = esp_xray_server::packet::decode_u32(&buf, pos);
                            pos = index;
                            xray.send(Message::TaskReadyBegin(task, ts_delta));
                        }
                        5 => {
                            pos += 1;
                            let (index, task) = esp_xray_server::packet::decode_u32(&buf, pos);
                            pos = index;
                            let (index, ts_delta) = esp_xray_server::packet::decode_u32(&buf, pos);
                            pos += index;
                            xray.send(Message::TaskReadyEnd(task, ts_delta));
                        }
                        6 => {
                            pos += 1;
                            let (index, ts_delta) = esp_xray_server::packet::decode_u32(&buf, pos);
                            pos = index;
                            xray.send(Message::SystemIdle(ts_delta));
                        }
                        _ => {
                            // shouldn't happen
                            pos += 1;
                        }
                    }
                }
            }
            // TODO handle disconnect command / commands in general
        }
    }
}
