use std::io::{Read, Write};

use crate::packet::{Cause, Event};

pub mod packet;

#[macro_export]
macro_rules! block {
    ($e:expr) => {
        loop {
            #[allow(unreachable_patterns)]
            match $e {
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) =>
                {
                    #[allow(unreachable_code)]
                    break Err(e)
                }
                Ok(x) => break Ok(x),
            }
        }
    };
}

#[derive(Debug, Clone, Copy)]
pub enum Error {
    UnknownCommand,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Disconnect(u32),
    IsrEnter(u8, u32),
    IsrExit(u32),
    TaskNew(u32, u32),
    TaskExecBegin(u32, u32),
    TaskExecEnd(u32),
    TaskReadyBegin(u32, u32),
    TaskReadyEnd(u32, u32),
    SystemIdle(u32),
}

pub trait Transport<IO>
where
    IO: Read + Write,
{
    fn hello(&self, io: &mut IO);

    fn skip_command_len(&self, io: &mut IO) -> bool;
}

#[derive(Default)]
pub struct TcpTransport {}

impl<IO> Transport<IO> for TcpTransport
where
    IO: Read + Write,
{
    fn hello(&self, io: &mut IO) {
        let mut buf = [0u8; 48];
        let _count = block!(io.read(&mut buf)).unwrap();

        // TODO we should check the host's HELLO

        io.write_all(&[
            b'S',
            b'E',
            b'G',
            b'G',
            b'E',
            b'R',
            b' ',
            b'S',
            b'y',
            b's',
            b't',
            b'e',
            b'm',
            b'V',
            b'i',
            b'e',
            b'w',
            b' ',
            b'V',
            b'0' + 3,
            b'.',
            b'0' + (0 / 10),
            b'0' + (0 % 10),
            b'.',
            b'0' + (0 / 10),
            b'0' + (0 % 10),
            b'\0',
            0,
            0,
            0,
            0,
            0,
        ])
        .unwrap();

        // AB sync
        io.write_all(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
            .unwrap();
    }

    fn skip_command_len(&self, io: &mut IO) -> bool {
        let mut buf = [0u8];
        match io.read_exact(&mut buf) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

// TODO UART

pub struct SystemViewTarget<T, IO>
where
    T: Transport<IO>,
    IO: Read + Write,
{
    transport: T,
    io: IO,
}

impl<T, IO> SystemViewTarget<T, IO>
where
    T: Transport<IO>,
    IO: Read + Write,
{
    pub fn new(transport: T, mut io: IO) -> Self {
        transport.hello(&mut io);

        let mut cmd = [0u8; 5];
        let mut out = [0u8; 32];

        // read start command
        while !transport.skip_command_len(&mut io) {}
        let _count = block!(io.read(&mut cmd)).unwrap();

        // should be answer to Command::Start
        let l = Event::TraceStart { ts_delta: 0 }.encode(&mut out).unwrap();
        io.write(&out[..l]).unwrap();

        // TODO get it from the target
        let l = Event::Init {
            sys_freq: 16000000,
            cpu_freq: 160000000,
            ram_base: 0x40000000,
            id_shift: 2,
            ts_delta: 1,
        }
        .encode(&mut out)
        .unwrap();
        io.write(&out[..l]).unwrap();

        let l = Event::SystimeCycles {
            time: 1000,
            ts_delta: 3,
        }
        .encode(&mut out)
        .unwrap();
        io.write_all(&out[..l]).unwrap();

        let l = Event::NumModules {
            modules: 0,
            ts_delta: 4,
        }
        .encode(&mut out)
        .unwrap();
        io.write_all(&out[..l]).unwrap();

        Self { transport, io }
    }

    pub fn process_incoming(&mut self) -> bool {
        let mut cmd = [0u8; 5];
        let transport = &mut self.transport;
        let io = &mut self.io;

        if transport.skip_command_len(io) {
            let _count = block!(io.read(&mut cmd)).unwrap();
            // for now assume anything from the host will be a Disconnect command
            // TODO send Event::TraceStop
            let mut out = [0u8; 32];
            let l = Event::TraceStop { ts_delta: 0 }.encode(&mut out).unwrap();
            io.write_all(&out[..l]).unwrap();

            return _count > 0;
        }

        false
    }

    pub fn send(&mut self, msg: Message) {
        #[cfg(feature = "log")]
        log::info!("Run...");

        let mut out = [0u8; 32];
        match msg {
            Message::IsrEnter(isr, ts_delta) => {
                let l = Event::IsrEnter { isr, ts_delta }.encode(&mut out).unwrap();
                self.io.write_all(&out[..l]).unwrap();
            }
            Message::IsrExit(ts_delta) => {
                let l = Event::IsrExit { ts_delta }.encode(&mut out).unwrap();
                self.io.write_all(&out[..l]).unwrap();
            }
            Message::Disconnect(ts_delta) => {
                // HOST disconnect
                let l = Event::TraceStop { ts_delta }.encode(&mut out).unwrap();
                self.io.write_all(&out[..l]).unwrap();
            }
            Message::TaskNew(task, ts_delta) => {
                let l = Event::TaskCreate { task, ts_delta }
                    .encode(&mut out)
                    .unwrap();
                self.io.write_all(&out[..l]).unwrap();
            }
            Message::TaskExecBegin(task, ts_delta) => {
                let l = Event::TaskStartExec { task, ts_delta }
                    .encode(&mut out)
                    .unwrap();
                self.io.write_all(&out[..l]).unwrap();
            }
            Message::TaskExecEnd(ts_delta) => {
                let l = Event::TaskStopExec { ts_delta }.encode(&mut out).unwrap();
                self.io.write_all(&out[..l]).unwrap();
            }
            Message::TaskReadyBegin(task, ts_delta) => {
                let l = Event::TaskStartReady { task, ts_delta }
                    .encode(&mut out)
                    .unwrap();
                self.io.write_all(&out[..l]).unwrap();
            }
            Message::TaskReadyEnd(task, ts_delta) => {
                let l = Event::TaskStopReady {
                    task,
                    cause: Cause::Idle,
                    ts_delta,
                }
                .encode(&mut out)
                .unwrap();
                self.io.write_all(&out[..l]).unwrap();
            }
            Message::SystemIdle(ts_delta) => {
                let l = Event::Idle { ts_delta }.encode(&mut out).unwrap();
                self.io.write_all(&out[..l]).unwrap();
            }
        }

        #[cfg(feature = "log")]
        log::info!("Done.");
    }
}
