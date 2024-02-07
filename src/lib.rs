#![no_std]
#![allow(async_fn_in_trait)]
#![feature(async_closure)]

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
    IO: embedded_io_async::Read + embedded_io_async::Write,
{
    async fn hello(&self, io: &mut IO);

    async fn skip_command_len(&self, io: &mut IO);
}

#[derive(Default)]
pub struct TcpTransport {}

impl<IO> Transport<IO> for TcpTransport
where
    IO: embedded_io_async::Read + embedded_io_async::Write,
{
    async fn hello(&self, io: &mut IO) {
        let mut buf = [0u8; 48];
        let _count = io.read(&mut buf).await.unwrap();

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
        .await
        .unwrap();

        // AB sync
        io.write_all(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
            .await
            .unwrap();
    }

    async fn skip_command_len(&self, io: &mut IO) {
        let mut buf = [0u8];
        io.read_exact(&mut buf).await.ok();
    }
}

// TODO UART

pub struct SystemViewTarget<T, IO>
where
    T: Transport<IO>,
    IO: embedded_io_async::Read + embedded_io_async::Write,
{
    transport: T,
    io: IO,
}

impl<T, IO> SystemViewTarget<T, IO>
where
    T: Transport<IO>,
    IO: embedded_io_async::Read + embedded_io_async::Write,
{
    pub async fn new(transport: T, mut io: IO) -> Self {
        transport.hello(&mut io).await;

        Self { transport, io }
    }

    pub async fn run(&mut self, receiver: Receiver<'_, CriticalSectionRawMutex, Message, 5>) {
        #[cfg(feature = "log")]
        log::info!("Run...");

        // TODO implement something "real" :)

        let mut cmd = [0u8; 5];
        let mut out = [0u8; 32];

        // read start command
        self.transport.skip_command_len(&mut self.io).await;
        let _count = self.io.read(&mut cmd).await.unwrap();

        // should be answer to Command::Start
        let l = Event::TraceStart { ts_delta: 0 }.encode(&mut out).unwrap();
        self.io.write(&out[..l]).await.unwrap();

        let l = Event::Init {
            sys_freq: 80000,
            cpu_freq: 160000,
            ram_base: 0x40000000,
            id_shift: 2,
            ts_delta: 1,
        }
        .encode(&mut out)
        .unwrap();
        self.io.write(&out[..l]).await.unwrap();

        let l = Event::SystimeCycles {
            time: 1000,
            ts_delta: 3,
        }
        .encode(&mut out)
        .unwrap();
        self.io.write_all(&out[..l]).await.unwrap();

        let l = Event::NumModules {
            modules: 0,
            ts_delta: 4,
        }
        .encode(&mut out)
        .unwrap();
        self.io.write_all(&out[..l]).await.unwrap();

        // everything is up now ... we can send events
        loop {
            let transport = &self.transport;
            let io = &mut self.io;
            let msg = futures_lite::future::race(
                (async || receiver.receive().await)(),
                (async || {
                    transport.skip_command_len(io).await;
                    let _count = io.read(&mut cmd).await.unwrap();
                    Message::Disconnect
                })(),
            )
            .await;

            match msg {
                Message::IsrEnter(isr) => {
                    let l = Event::IsrEnter { isr, ts_delta: 30 }
                        .encode(&mut out)
                        .unwrap();
                    self.io.write_all(&out[..l]).await.unwrap();
                }
                Message::IsrExit => {
                    let l = Event::IsrExit { ts_delta: 10 }.encode(&mut out).unwrap();
                    self.io.write_all(&out[..l]).await.unwrap();
                }
                Message::Disconnect => {
                    // HOST disconnect
                    let l = Event::TraceStop { ts_delta: 100 }.encode(&mut out).unwrap();
                    self.io.write_all(&out[..l]).await.unwrap();
                    break;
                }
            }
        }

        #[cfg(feature = "log")]
        log::info!("Done.");
    }
}
