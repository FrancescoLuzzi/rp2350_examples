#![no_std]
#![no_main]
use core::sync::atomic::{AtomicBool, Ordering};

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::peripherals::{PIO0, USB};
use embassy_rp::usb::{Driver as UsbDriver, InterruptHandler as UsbInterruptHandler};
use embassy_rp::{
    pio::{InterruptHandler as PioInterruptHandler, Pio},
    pio_programs::rotary_encoder::{Direction, PioEncoder, PioEncoderProgram},
};
use embassy_time::Timer;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State as CdcAcmState};
use embassy_usb::class::hid::{HidReaderWriter, ReportId, RequestHandler, State as HidState};
use embassy_usb::control::OutResponse;
use embassy_usb::{Builder, Config, Handler};
use panic_probe as _;
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
});

#[embassy_executor::task]
async fn rotary_encoder_task(mut encoder: PioEncoder<'static, PIO0, 0>) {
    let mut count = 0;
    loop {
        log::info!("Count: {}", count);
        count += match encoder.read().await {
            Direction::Clockwise => 1,
            Direction::CounterClockwise => -1,
        };
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO0, Irqs);

    let prg = PioEncoderProgram::new(&mut common);
    let encoder0 = PioEncoder::new(&mut common, sm0, p.PIN_4, p.PIN_5, &prg);

    spawner.must_spawn(rotary_encoder_task(encoder0));
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

    let mut hid_state = HidState::new();
    let mut serial_state = CdcAcmState::new();

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
    let hid = HidReaderWriter::<_, 1, 8>::new(&mut builder, &mut hid_state, config);
    let serial_class = CdcAcmClass::new(&mut builder, &mut serial_state, 64);

    // Create a class for the logger
    let log_fut = embassy_usb_logger::with_class!(1024, log::LevelFilter::Info, serial_class);

    // Build the builder.
    let mut usb = builder.build();

    // Run the USB device.
    let usb_fut = usb.run();

    let mut col0 = Output::new(p.PIN_12, Level::High);
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
    let mut old_report = [0, 0, 0, 0, 0, 0];

    // Do stuff with the class!
    let in_fut = async {
        loop {
            if report.keycodes.iter().all(|x| x == &0) {
                led.set_low();
            } else {
                led.set_high();
            }

            core::mem::swap(&mut report.keycodes, &mut old_report);
            report.keycodes = [0, 0, 0, 0, 0, 0];
            col0.set_low();
            Timer::after_nanos(100).await;
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
            Timer::after_nanos(100).await;
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
            if report.keycodes == old_report {
                continue;
            }
            log::info!("{:?}", report.keycodes);
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
    join(usb_fut, join(log_fut, join(in_fut, out_fut))).await;
    // join(usb_fut, join(in_fut, out_fut)).await;
}

struct MyRequestHandler {}

impl RequestHandler for MyRequestHandler {
    fn get_report(&mut self, id: ReportId, _buf: &mut [u8]) -> Option<usize> {
        log::info!("Get report for {:?}", id);
        None
    }

    fn set_report(&mut self, id: ReportId, data: &[u8]) -> OutResponse {
        // info!("Set report for {:?}: {=[u8]}", id, data);
        log::info!("Set report for {:?}: {:?}", id, data);
        OutResponse::Accepted
    }

    fn set_idle_ms(&mut self, id: Option<ReportId>, dur: u32) {
        log::info!("Set idle rate for {:?} to {:?}", id, dur);
    }

    fn get_idle_ms(&mut self, id: Option<ReportId>) -> Option<u32> {
        log::info!("Get idle rate for {:?}", id);
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
            log::info!("Device enabled");
        } else {
            log::info!("Device disabled");
        }
    }

    fn reset(&mut self) {
        self.configured.store(false, Ordering::Relaxed);
        log::info!("Bus reset, the Vbus current limit is 100mA");
    }

    fn addressed(&mut self, addr: u8) {
        self.configured.store(false, Ordering::Relaxed);
        log::info!("USB address set to: {}", addr);
    }

    fn configured(&mut self, configured: bool) {
        self.configured.store(configured, Ordering::Relaxed);
        if configured {
            log::info!(
                "Device configured, it may now draw up to the configured current limit from Vbus."
            )
        } else {
            log::info!("Device is no longer configured, the Vbus current limit is 100mA.");
        }
    }
}
