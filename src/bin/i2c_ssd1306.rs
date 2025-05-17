#![no_std]
#![no_main]

use defmt::*;
use embassy_rp::i2c::InterruptHandler;
use embedded_graphics::{
    image::{Image, ImageRaw},
    pixelcolor::BinaryColor,
    prelude::*,
};
use ssd1306::{I2CDisplayInterface, Ssd1306Async, prelude::*};
use {defmt_rtt as _, panic_probe as _};

embassy_rp::bind_interrupts!(struct Irqs {
    I2C1_IRQ => InterruptHandler<embassy_rp::peripherals::I2C1>;
});

#[embassy_executor::main]
async fn main(_task_spawner: embassy_executor::Spawner) {
    let p = embassy_rp::init(Default::default());
    let sda = p.PIN_14;
    let scl = p.PIN_15;
    let config = embassy_rp::i2c::Config::default();
    let i2c = embassy_rp::i2c::I2c::new_async(p.I2C1, scl, sda, Irqs, config);
    // wait for sensors to initialize
    embassy_time::Timer::after(embassy_time::Duration::from_millis(100)).await;

    let interface = I2CDisplayInterface::new(i2c);
    let mut display_i2c = Ssd1306Async::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();

    // Init and reset both displays as needed
    display_i2c.init().await.unwrap();

    let raw: ImageRaw<BinaryColor> = ImageRaw::new(include_bytes!("../../rust.bmp"), 64);

    for i in (0..=64).chain((0..64).rev()).cycle() {
        let top_left = Point::new(i, 0);
        let im = Image::new(&raw, top_left);

        im.draw(&mut display_i2c).unwrap();
        display_i2c.flush().await.unwrap();
        display_i2c.clear(BinaryColor::Off).unwrap();
    }
}
