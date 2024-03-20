#![no_std]

use core::cell::RefCell;

use critical_section::Mutex;
use rtos_trace::RtosTrace;
use rtt_target::ChannelMode::NoBlockSkip;
use rtt_target::{rtt_init, UpChannel};

struct RtosTraceImpl;

enum Event {
    TaskNew = 1,
    TaskExecBegin,
    TaskExecEnd,
    TaskReadyBegin,
    TaskReadyEnd,
    SystemIdle,
}

impl RtosTrace for RtosTraceImpl {
    fn task_new(id: u32) {
        let mut buffer = [0u8; 16];
        buffer[0] = Event::TaskNew as u8;
        let pos = encode_u32(id, &mut buffer, 1);
        let pos = encode_u32(get_ts_delta(), &mut buffer, pos);
        post(&buffer[..pos]);
    }

    fn task_exec_begin(id: u32) {
        let mut buffer = [0u8; 16];
        buffer[0] = Event::TaskExecBegin as u8;
        let pos = encode_u32(id, &mut buffer, 1);
        let pos = encode_u32(get_ts_delta(), &mut buffer, pos);
        post(&buffer[..pos]);
    }

    fn task_exec_end() {
        let mut buffer = [0u8; 16];
        buffer[0] = Event::TaskExecEnd as u8;
        let pos = encode_u32(get_ts_delta(), &mut buffer, 1);
        post(&buffer[..pos]);
    }

    fn task_ready_begin(id: u32) {
        let mut buffer = [0u8; 16];
        buffer[0] = Event::TaskReadyBegin as u8;
        let pos = encode_u32(id, &mut buffer, 1);
        let pos = encode_u32(get_ts_delta(), &mut buffer, pos);
        post(&buffer[..pos]);
    }

    fn task_ready_end(id: u32) {
        let mut buffer = [0u8; 16];
        buffer[0] = Event::TaskReadyEnd as u8;
        let pos = encode_u32(id, &mut buffer, 1);
        let pos = encode_u32(get_ts_delta(), &mut buffer, pos);
        post(&buffer[..pos]);
    }

    fn system_idle() {
        let mut buffer = [0u8; 16];
        buffer[0] = Event::SystemIdle as u8;
        let pos = encode_u32(get_ts_delta(), &mut buffer, 1);
        post(&buffer[..pos]);
    }

    fn task_send_info(_id: u32, _info: rtos_trace::TaskInfo) {}

    fn task_terminate(_id: u32) {}

    fn isr_enter() {}

    fn isr_exit() {}

    fn isr_exit_to_scheduler() {}

    fn marker(_id: u32) {}

    fn marker_begin(_id: u32) {}

    fn marker_end(_id: u32) {}
}

rtos_trace::global_trace! {RtosTraceImpl}

static CHANNEL: Mutex<RefCell<Option<UpChannel>>> = Mutex::new(RefCell::new(None));
static LAST_TS: Mutex<RefCell<u64>> = Mutex::new(RefCell::new(0));

fn get_ts_delta() -> u32 {
    critical_section::with(|cs| {
        let last = LAST_TS.take(cs);
        let now = esp_hal::systimer::SystemTimer::now();
        LAST_TS.replace(cs, now);

        if last == 0 {
            0
        } else {
            (now - last) as u32
        }
    })
}

fn encode_u32(mut value: u32, buffer: &mut [u8], mut count: usize) -> usize {
    while value > 0x7F {
        buffer[count] = (value | 0x80) as u8;
        count += 1;
        value >>= 7;
    }
    buffer[count] = value as u8;
    count += 1;

    count
}

fn post(data: &[u8]) {
    critical_section::with(|cs| {
        if CHANNEL.borrow_ref_mut(cs).is_none() {
            let channels = rtt_init! {
                up: {
                    0: {
                        size: 1024,
                        mode: NoBlockSkip,
                        name: "Xray"
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
