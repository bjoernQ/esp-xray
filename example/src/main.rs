#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl, peripherals::Peripherals, prelude::*, system::SystemControl,
    timer::timg::TimerGroup,
};
use esp_xray as _;

#[embassy_executor::task]
async fn run() {
    loop {
        esp_println::println!("Hello world");
        for _ in 0..10000 {}
        Timer::after(Duration::from_millis(10)).await;
    }
}

#[main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();

    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    let mut rng = esp_hal::rng::Rng::new(peripherals.RNG);

    let timg0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    esp_hal_embassy::init(&clocks, timg0.timer0);

    spawner.spawn(run()).ok();

    loop {
        esp_println::println!("Bing!");
        for _ in 0..22000 {}
        Timer::after(Duration::from_millis(
            rng.random() as u64 % 10,
        ))
        .await;
    }
}
