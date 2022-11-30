#![no_std]
#![no_main]

use cortex_m::delay::Delay;
use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use panic_probe as _;
use rp_pico::{entry, Pins, XOSC_CRYSTAL_FREQ};

use rp_pico::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    usb::UsbBus,
    watchdog::Watchdog,
};

use usb_device::{class_prelude::*, prelude::*};

use usbd_hid::descriptor::generator_prelude::*;
use usbd_hid::descriptor::KeyboardReport;
use usbd_hid::hid_class::HIDClass;

const USB_HOST_POLL_MS: u8 = 10;

const KEY_I: u8 = 0x0c;
const KEY_ESC: u8 = 0x29;

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    let clocks = init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let usb_bus = UsbBusAllocator::new(rp_pico::hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut usb_hid = HIDClass::new(&usb_bus, KeyboardReport::desc(), USB_HOST_POLL_MS);

    // Create a USB device with a fake VID and PID
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27da))
        .manufacturer("Chris Price")
        .product("Vim Clutch")
        .serial_number("0")
        .device_class(0)
        .build();

    let mut delay = Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut led_pin = pins.led.into_push_pull_output();
    let switch_pin = pins.gpio0.into_pull_up_input();
    let mut switch_state = switch_pin.is_low().unwrap();

    loop {
        usb_dev.poll(&mut [&mut usb_hid]);

        let previous_switch_state = switch_state;
        switch_state = switch_pin.is_low().unwrap();

        match (previous_switch_state, switch_state) {
            (true, false) => {
                info!("normal mode!");
                led_pin.set_low().unwrap();

                send_key_press(&usb_hid, &mut delay, KEY_ESC);
            }
            (false, true) => {
                info!("insert mode!");
                led_pin.set_high().unwrap();

                send_key_press(&usb_hid, &mut delay, KEY_I);
            }
            _ => {}
        }
    }
}

fn send_key_press(usb_hid: &HIDClass<UsbBus>, delay: &mut Delay, key_code: u8) {
    let mut keyboard_report = KeyboardReport {
        modifier: 0,
        reserved: 0,
        leds: 0,
        keycodes: [0; 6],
    };
    keyboard_report.keycodes[0] = key_code;
    usb_hid.push_input(&keyboard_report).unwrap();
    delay.delay_ms(USB_HOST_POLL_MS.into());

    keyboard_report.keycodes[0] = 0;
    usb_hid.push_input(&keyboard_report).unwrap();
    delay.delay_ms(USB_HOST_POLL_MS.into());
}
