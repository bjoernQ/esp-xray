#![no_std]

use core::cell::RefCell;

use critical_section::Mutex;
use rtos_trace::RtosTrace;
use rtt_target::ChannelMode::NoBlockSkip;
use rtt_target::{rtt_init, UpChannel};

struct RtosTraceImpl;

impl RtosTrace for RtosTraceImpl {
    fn task_new(id: u32) {
        // println!("@@@ task new {id}");
        post(&[1]);
    }

    fn task_send_info(id: u32, info: rtos_trace::TaskInfo) {
        todo!()
    }

    fn task_terminate(id: u32) {
        todo!()
    }

    fn task_exec_begin(id: u32) {
        // println!("@@@ task exec begin {id}");
        post(&[2]);
    }

    fn task_exec_end() {
        // println!("@@@ task exec end");
        post(&[3]);
    }

    fn task_ready_begin(id: u32) {
        // println!("@@@ task ready begin {id}");
        post(&[4]);
    }

    fn task_ready_end(id: u32) {
        // println!("@@@ task ready end {id}");
        post(&[5]);
    }

    fn system_idle() {
        // println!("@@@ idle");
        post(&[6]);
    }

    fn isr_enter() {
        // unused in embassy
    }

    fn isr_exit() {
        // unused in embassy
    }

    fn isr_exit_to_scheduler() {
        // unused in embassy
    }

    fn marker(id: u32) {
        // unused in embassy ???
    }

    fn marker_begin(id: u32) {
        // unused in embassy ???
    }

    fn marker_end(id: u32) {
        // unused in embassy ???
    }
}

rtos_trace::global_trace! {RtosTraceImpl}

static CHANNEL: Mutex<RefCell<Option<UpChannel>>> = Mutex::new(RefCell::new(None));

fn post(data: &[u8]) {
    critical_section::with(|cs| {
        if CHANNEL.borrow_ref_mut(cs).is_none() {
            let channels = rtt_init! {
                up: {
                    0: { // channel number
                        size: 1024, // buffer size in bytes
                        mode: NoBlockSkip, // mode (optional, default: NoBlockSkip, see enum ChannelMode)
                        name: "Xray" // name (optional, default: no name)
                    }
                }
            };
            CHANNEL.borrow_ref_mut(cs).replace(channels.up.0);
        }
        let mut channel = CHANNEL.borrow_ref_mut(cs);
        let channel = channel.as_mut().unwrap();
        channel.write(data);
    });
}
