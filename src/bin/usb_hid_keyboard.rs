#![no_std]
#![no_main]

use core::sync::atomic::{AtomicBool, Ordering};

use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver as UsbDriver, InterruptHandler};
use embassy_time::Timer;
use embassy_usb::class::hid::{HidReaderWriter, ReportId, RequestHandler, State as HidState};
use embassy_usb::control::OutResponse;
use embassy_usb::{Builder, Config, Handler};
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    // Create the driver, from the HAL.
    let driver = UsbDriver::new(p.USB, Irqs);
    let mut led = Output::new(p.PIN_25, Level::Low);

    // Create embassy-usb Config
    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("Embassy");
    config.product = Some("HID keyboard example");
    config.serial_number = Some("12345678");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    // You can also add a Microsoft OS descriptor.
    let mut msos_descriptor = [0; 256];
    let mut control_buf = [0; 64];
    let mut request_handler = MyRequestHandler {};
    let mut device_handler = MyDeviceHandler::new();

    let mut state = HidState::new();

    let mut builder = Builder::new(
        driver,
        config,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut msos_descriptor,
        &mut control_buf,
    );

    builder.handler(&mut device_handler);

    // Create classes on the builder.
    let config = embassy_usb::class::hid::Config {
        report_descriptor: KeyboardReport::desc(),
        request_handler: None,
        poll_ms: 60,
        max_packet_size: 64,
    };
    let hid = HidReaderWriter::<_, 1, 8>::new(&mut builder, &mut state, config);

    // Build the builder.
    let mut usb = builder.build();

    // Run the USB device.
    let usb_fut = usb.run();

    let mut col0 = Output::new(p.PIN_11, Level::High);
    let mut col1 = Output::new(p.PIN_13, Level::High);
    // Set up the signal pin that will be used to trigger the keyboard.
    let mut row0 = Input::new(p.PIN_14, Pull::Up);
    let mut row1 = Input::new(p.PIN_15, Pull::Up);
    row0.set_schmitt(true);
    row1.set_schmitt(true);

    let (reader, mut writer) = hid.split();

    let mut report = KeyboardReport {
        keycodes: [0, 0, 0, 0, 0, 0],
        leds: 0,
        modifier: 0,
        reserved: 0,
    };

    // Do stuff with the class!
    let in_fut = async {
        loop {
            if report.keycodes.iter().all(|x| *x == 0) {
                led.set_low();
            } else {
                led.set_high();
            }
            col0.set_low();
            Timer::after_micros(500).await;
            if row0.is_low() {
                if let Some(index) = report.keycodes.iter_mut().position(|x| *x == 0) {
                    report.keycodes[index] = 4;
                }
            } else if let Some(index) = report.keycodes.iter_mut().position(|x| *x == 4) {
                report.keycodes[index] = 0;
            }
            if row1.is_low() {
                if let Some(index) = report.keycodes.iter_mut().position(|x| *x == 0) {
                    report.keycodes[index] = 5;
                }
            } else if let Some(index) = report.keycodes.iter_mut().position(|x| *x == 5) {
                report.keycodes[index] = 0;
            }
            col0.set_high();
            col1.set_low();
            Timer::after_micros(100).await;
            if row0.is_low() {
                if let Some(index) = report.keycodes.iter_mut().position(|x| *x == 0) {
                    report.keycodes[index] = 6;
                }
            } else if let Some(index) = report.keycodes.iter_mut().position(|x| *x == 6) {
                report.keycodes[index] = 0;
            }
            if row1.is_low() {
                if let Some(index) = report.keycodes.iter_mut().position(|x| *x == 0) {
                    report.keycodes[index] = 7;
                }
            } else if let Some(index) = report.keycodes.iter_mut().position(|x| *x == 7) {
                report.keycodes[index] = 0;
            }
            col1.set_high();
            match writer.write_serialize(&report).await {
                Ok(()) => {}
                Err(e) => warn!("Failed to send report: {:?}", e),
            };
        }
    };

    let out_fut = async {
        reader.run(false, &mut request_handler).await;
    };

    // Run everything concurrently.
    // If we had made everything `'static` above instead, we could do this using separate tasks instead.
    join(usb_fut, join(in_fut, out_fut)).await;
}

struct MyRequestHandler {}

impl RequestHandler for MyRequestHandler {
    fn get_report(&mut self, id: ReportId, _buf: &mut [u8]) -> Option<usize> {
        info!("Get report for {:?}", id);
        None
    }

    fn set_report(&mut self, id: ReportId, data: &[u8]) -> OutResponse {
        info!("Set report for {:?}: {=[u8]}", id, data);
        OutResponse::Accepted
    }

    fn set_idle_ms(&mut self, id: Option<ReportId>, dur: u32) {
        info!("Set idle rate for {:?} to {:?}", id, dur);
    }

    fn get_idle_ms(&mut self, id: Option<ReportId>) -> Option<u32> {
        info!("Get idle rate for {:?}", id);
        None
    }
}

struct MyDeviceHandler {
    configured: AtomicBool,
}

impl MyDeviceHandler {
    fn new() -> Self {
        MyDeviceHandler {
            configured: AtomicBool::new(false),
        }
    }
}

impl Handler for MyDeviceHandler {
    fn enabled(&mut self, enabled: bool) {
        self.configured.store(false, Ordering::Relaxed);
        if enabled {
            info!("Device enabled");
        } else {
            info!("Device disabled");
        }
    }

    fn reset(&mut self) {
        self.configured.store(false, Ordering::Relaxed);
        info!("Bus reset, the Vbus current limit is 100mA");
    }

    fn addressed(&mut self, addr: u8) {
        self.configured.store(false, Ordering::Relaxed);
        info!("USB address set to: {}", addr);
    }

    fn configured(&mut self, configured: bool) {
        self.configured.store(configured, Ordering::Relaxed);
        if configured {
            info!(
                "Device configured, it may now draw up to the configured current limit from Vbus."
            )
        } else {
            info!("Device is no longer configured, the Vbus current limit is 100mA.");
        }
    }
}
