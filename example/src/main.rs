#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::timer::TimerGroup;
use esp_hal::{clock::ClockControl, embassy, peripherals::Peripherals, prelude::*};
use esp_xray as _;

#[embassy_executor::task]
async fn run() {
    loop {
        esp_println::println!(
            "Hello world from embassy using esp-hal-async! {}",
            esp_hal::systimer::SystemTimer::now()
        );
        for _ in 0..10000 {}
        Timer::after(Duration::from_millis(10)).await;
    }
}

#[main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    let mut rng = esp_hal::rng::Rng::new(peripherals.RNG);

    let timg0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timg0);

    spawner.spawn(run()).ok();

    loop {
        esp_println::println!("Bing! {}", esp_hal::systimer::SystemTimer::now());
        for _ in 0..22000 {}
        Timer::after(Duration::from_millis( rng.random() as u64 / ( (u32::MAX / 100)) as u64)   ).await;
    }
}
