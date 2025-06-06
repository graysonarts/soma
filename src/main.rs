#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::gpio;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_time::{Duration, Timer};
use gpio::{Level, Output};
use {defmt_rtt as _, panic_probe as _};

use cyw43_pio::{PioSpi, DEFAULT_CLOCK_DIVIDER};
use defmt::*;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

type ButtonId = u8;
const BUTTON_COUNT: usize = 12;
const CHANNEL_SIZE: usize = BUTTON_COUNT;
static CHANNEL: Channel<ThreadModeRawMutex, ButtonId, CHANNEL_SIZE> = Channel::new();

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task(pool_size = 2)]
async fn button_task(
    mut button: gpio::Input<'static>,
    button_id: ButtonId,
    sender: Sender<'static, ThreadModeRawMutex, ButtonId, CHANNEL_SIZE>,
) {
    loop {
        button.wait_for_falling_edge().await;
        Timer::after_millis(10).await; // Debounce
        if button.is_high() {
            continue;
        }
        sender.send(button_id).await;
    }
}

#[embassy_executor::task]
async fn led_task(
    mut leds: [gpio::Output<'static>; BUTTON_COUNT],
    receiver: Receiver<'static, ThreadModeRawMutex, ButtonId, CHANNEL_SIZE>,
) {
    loop {
        let button_id = receiver.receive().await;
        for i in 0..BUTTON_COUNT {
            match i {
                button_id => leds[i].set_low(),
                _ => leds[i].set_high(),
            }
        }
    }
}

macro_rules! button {
    ($pin:expr) => {
        gpio::Input::new($pin, gpio::Pull::Up)
    };
}

macro_rules! led {
    ($pin:expr) => {
        gpio::Output::new($pin, Level::Low)
    };
}

macro_rules! audio_out {
    ($pin:expr) => {
        gpio::Output::new($pin, Level::High)
    };
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting main");
    // Initialise Peripherals
    let p = embassy_rp::init(Default::default());

    info!("Initializing buttons");
    let buttons = [
        button!(p.PIN_16),
        button!(p.PIN_17),
        button!(p.PIN_18),
        button!(p.PIN_19),
        button!(p.PIN_20),
        button!(p.PIN_21),
        button!(p.PIN_22),
        button!(p.PIN_23),
        button!(p.PIN_24),
    ];
    buttons.into_iter().enumerate().for_each(|(i, button)| {
        spawner
            .spawn(button_task(button, i as ButtonId, CHANNEL.sender()))
            .unwrap();
    });

    info!("Initializing leds");
    let leds = [
        led!(p.PIN_0),
        led!(p.PIN_1),
        led!(p.PIN_2),
        led!(p.PIN_3),
        led!(p.PIN_4),
        led!(p.PIN_5),
        led!(p.PIN_6),
        led!(p.PIN_7),
        led!(p.PIN_8),
        led!(p.PIN_9),
        led!(p.PIN_10),
        led!(p.PIN_11),
    ];
    spawner.spawn(led_task(leds, CHANNEL.receiver())).unwrap();

    info!("Initializing audio control");
    let audio_controls = [
        audio_out!(p.PIN_12),
        audio_out!(p.PIN_13),
        audio_out!(p.PIN_14),
        audio_out!(p.PIN_15),
    ];

    loop {
        let button_id = CHANNEL.receive().await;
        info!("Button pressed: {}", button_id);

        // TODO: Handle the audio switching
    }
}
