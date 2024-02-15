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

    loop {
        let mut buf = [0u8; 1024];
        let x = up_channel.read(&mut core, &mut buf).unwrap();
        if x != 0 {
            println!("done {x} {:x?}", &buf[..x]);
        }
    }
}
