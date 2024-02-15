#![feature(type_alias_impl_trait)]

use async_net::TcpListener;
use embassy_executor::Executor;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Receiver, Sender},
};
use embassy_time::Timer;
use esp_xray::Message;
use futures_lite::{AsyncReadExt, AsyncWriteExt, StreamExt};
use static_cell::{make_static, StaticCell};

static EXECUTOR: StaticCell<Executor> = StaticCell::new();

fn main() {
    pretty_env_logger::init();

    let executor = EXECUTOR.init(Executor::new());

    let channel = Channel::<CriticalSectionRawMutex, Message, 5>::new();
    let channel = make_static!(channel);
    let receiver = channel.receiver();
    let sender = channel.sender();

    executor.run(|spawner| {
        spawner.spawn(xray_task(receiver)).unwrap();
        spawner.spawn(producer_task(sender)).unwrap();
    });
}

#[embassy_executor::task]
async fn producer_task(sender: Sender<'static, CriticalSectionRawMutex, Message, 5>) {
    let mut isr = 0;
    loop {
        Timer::after_secs(1).await;
        sender.send(Message::IsrEnter(isr)).await;
        Timer::after_secs(1).await;
        sender.send(Message::IsrExit).await;

        isr = (isr + 1) % 16;
    }
}

#[embassy_executor::task]
async fn xray_task(receiver: Receiver<'static, CriticalSectionRawMutex, Message, 5>) {
    let listener = TcpListener::bind(("127.0.0.1", 8080)).await.unwrap();

    let mut incoming = listener.incoming();

    while let Some(stream) = incoming.next().await {
        let stream = stream.unwrap();
        let adapter = Adapter::new(stream);

        let mut xray =
            esp_xray::SystemViewTarget::new(esp_xray::TcpTransport::default(), adapter).await;
        xray.run(receiver).await;
    }
}

struct Adapter {
    stream: async_net::TcpStream,
}

impl Adapter {
    pub fn new(stream: async_net::TcpStream) -> Self {
        Self { stream }
    }
}

#[derive(Debug)]
struct AdapterError {}

impl embedded_io_async::Error for AdapterError {
    fn kind(&self) -> embedded_io_async::ErrorKind {
        embedded_io_async::ErrorKind::Other
    }
}

impl embedded_io_async::ErrorType for Adapter {
    type Error = AdapterError;
}

impl embedded_io_async::Read for Adapter {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.stream.read(buf).await.map_err(|_| AdapterError {})
    }
}

impl embedded_io_async::Write for Adapter {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let l = self.stream.write(buf).await.map_err(|_| AdapterError {})?;
        self.stream.flush().await.map_err(|_| AdapterError {})?;
        Ok(l)
    }
}
