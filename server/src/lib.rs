use std::io::{Read, Write};

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Receiver};

use crate::packet::Event;

pub mod packet;

#[derive(Debug, Clone, Copy)]
pub enum Error {
    UnknownCommand,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Disconnect,
    IsrEnter(u8),
    IsrExit,
}

pub trait Transport<IO>
where
    IO: Read + Write,
{
    fn hello(&self, io: &mut IO);

    fn skip_command_len(&self, io: &mut IO);
}

#[derive(Default)]
pub struct TcpTransport {}

impl<IO> Transport<IO> for TcpTransport
where
    IO: Read + Write,
{
    fn hello(&self, io: &mut IO) {
        let mut buf = [0u8; 48];
        let _count = io.read(&mut buf).unwrap();

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

    fn skip_command_len(&self, io: &mut IO) {
        let mut buf = [0u8];
        io.read_exact(&mut buf).ok();
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
        transport.skip_command_len(&mut io);
        let _count = io.read(&mut cmd).unwrap();

        // should be answer to Command::Start
        let l = Event::TraceStart { ts_delta: 0 }.encode(&mut out).unwrap();
        io.write(&out[..l]).unwrap();

        let l = Event::Init {
            sys_freq: 80000,
            cpu_freq: 160000,
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

    pub fn send(&mut self, msg: Message) {
        #[cfg(feature = "log")]
        log::info!("Run...");

        let mut cmd = [0u8; 5];
        let mut out = [0u8; 32];

        let transport = &self.transport;
        let io = &mut self.io;

        match msg {
            Message::IsrEnter(isr) => {
                let l = Event::IsrEnter { isr, ts_delta: 30 }
                    .encode(&mut out)
                    .unwrap();
                self.io.write_all(&out[..l]).unwrap();
            }
            Message::IsrExit => {
                let l = Event::IsrExit { ts_delta: 10 }.encode(&mut out).unwrap();
                self.io.write_all(&out[..l]).unwrap();
            }
            Message::Disconnect => {
                // HOST disconnect
                let l = Event::TraceStop { ts_delta: 100 }.encode(&mut out).unwrap();
                self.io.write_all(&out[..l]).unwrap();
            }
        }

        #[cfg(feature = "log")]
        log::info!("Done.");
    }
}
