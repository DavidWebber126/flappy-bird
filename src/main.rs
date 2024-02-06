#![no_main]
#![no_std]

use cortex_m_rt::entry;
use embedded_hal::{digital::v2::InputPin, blocking::delay::DelayMs};
//use rtt_target::{rtt_init_print, rprintln};
//use panic_rtt_target as _;
use panic_halt as _;
use defmt_rtt as _;

use core::cell::RefCell;
use cortex_m::interrupt::Mutex;

use microbit::{
    //Peripherals,
    display::blocking::Display,
    hal::{
        Timer,
        Rng,
        clocks::Clocks,
        gpio,
        prelude::OutputPin,
        pwm,
        rtc::{Rtc, RtcInterrupt},
        time::Hertz,
    },
    pac::{self, interrupt},
};

#[derive(Debug)]
pub enum Pipe {
    Low,
    MidLow,
    HighLow,
    High,
}

static RTC: Mutex<RefCell<Option<Rtc<pac::RTC0>>>> = Mutex::new(RefCell::new(None));
static SPEAKER: Mutex<RefCell<Option<pwm::Pwm<pac::PWM0>>>> = Mutex::new(RefCell::new(None));
static SPEAKER_OFF: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));

#[entry]
fn main() -> ! {
    //rtt_init_print!();

    let mut board = microbit::Board::take().unwrap();
    //let peripherals = Peripherals::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);
    let mut display = Display::new(board.display_pins);

    let button_a = board.buttons.button_a;
    let button_b = board.buttons.button_b;

    let mut rng = Rng::new(board.RNG);
    
    let mut bird_coord = (2, 1);

    let mut game_over = false;

    cortex_m::interrupt::free(move |cs| {
        // NB: The LF CLK pin is used by the speaker
        let _clocks = Clocks::new(board.CLOCK)
            .enable_ext_hfosc()
            .set_lfclk_src_synth()
            .start_lfclk();

        let mut rtc = Rtc::new(board.RTC0, 511).unwrap();
        rtc.enable_counter();
        rtc.enable_interrupt(RtcInterrupt::Tick, Some(&mut board.NVIC));
        rtc.enable_event(RtcInterrupt::Tick);

        *RTC.borrow(cs).borrow_mut() = Some(rtc);

        let mut speaker_pin = board.speaker_pin.into_push_pull_output(gpio::Level::High);
        let _ = speaker_pin.set_low();

        // Use the PWM peripheral to generate a waveform for the speaker
        let speaker = pwm::Pwm::new(board.PWM0);
        speaker
            // output the waveform on the speaker pin
            .set_output_pin(pwm::Channel::C0, speaker_pin.degrade())
            // Use prescale by 16 to achive darker sounds
            .set_prescaler(pwm::Prescaler::Div128)
            // Initial frequency
            .set_period(Hertz(1u32))
            // Configure for up and down counter mode
            .set_counter_mode(pwm::CounterMode::UpAndDown)
            // Set maximum duty cycle
            .set_max_duty(32767)
            // enable PWM
            .enable();

        speaker
            .set_seq_refresh(pwm::Seq::Seq0, 0)
            .set_seq_end_delay(pwm::Seq::Seq0, 0);

        // Configure 50% duty cycle
        let max_duty = speaker.max_duty();
        speaker.set_duty_on_common(max_duty / 2);

        *SPEAKER.borrow(cs).borrow_mut() = Some(speaker);

        // Configure RTC interrupt
        unsafe {
            pac::NVIC::unmask(pac::Interrupt::RTC0);
        }
        pac::NVIC::unpend(pac::Interrupt::RTC0);
    });
    
    //rprintln!("Entering main loop.");
    loop {
        let mut counter = 0;
        let mut pipes:[usize; 5] = [0,0,0,0,0];

        let mut screen = [
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0],
        ];

        while !game_over {
            // move pipes along and remove the pipe in first column
            pipes[0] = 0;
            pipes.rotate_right(4);

            // creates new pipes
            // counter to space out pipes on the screen (make gap between two leds)
            if counter % 4 == 0 {
                // reset counter to 1 and spawn new pipe
                counter = 1;

                let new_pipe = rng.random_u32() % 5;

                // update pipe array
                pipes[4] = new_pipe as usize;

            };

            // draw current pipes on screen
            for (pos, pipe) in pipes.iter().enumerate() {
                match pipe {
                    0 => {
                        screen[0][pos] = 0;
                        screen[1][pos] = 0;
                        screen[2][pos] = 0;
                        screen[3][pos] = 0;
                        screen[4][pos] = 0;
                    },
                    1 => {
                        screen[0][pos] = 1;
                        screen[1][pos] = 1;
                        screen[2][pos] = 1;
                        screen[3][pos] = 0;
                        screen[4][pos] = 0;
                    },
                    2 => {
                        screen[0][pos] = 1;
                        screen[1][pos] = 1;
                        screen[2][pos] = 0;
                        screen[3][pos] = 0;
                        screen[4][pos] = 1;
                    },
                    3 => {
                        screen[0][pos] = 1;
                        screen[1][pos] = 0;
                        screen[2][pos] = 0;
                        screen[3][pos] = 1;
                        screen[4][pos] = 1;
                    },
                    4 => {
                        screen[0][pos] = 0;
                        screen[1][pos] = 0;
                        screen[2][pos] = 1;
                        screen[3][pos] = 1;
                        screen[4][pos] = 1;
                    },
                    _ => {},
                }
            }

            // If user is pressing button, the bird jumps otherwise it falls
            // Note "falling" increments the x coordinate by one and "jumping" decrements
            if (button_a.is_low().unwrap()) || (button_b.is_low().unwrap()) {
                match bird_coord.0 {
                    0 => bird_coord.0 = bird_coord.0,
                    _ => bird_coord.0 -= 1,
                };
                
                // Put jumped bird on the screen
                screen[bird_coord.0][bird_coord.1] = 1;
                //rprintln!("bird jumped");

                //timer.delay_ms(250 as u16);

            } else {
                match bird_coord.0 {
                    4 => {
                        game_over = true;
                    },
                    _ => bird_coord.0 = bird_coord.0 + 1,
                }
                
                // bird falls one step
                screen[bird_coord.0][bird_coord.1] = 1;
                //rprintln!("bird fell");

                // timer.delay_ms(500 as u16);
            }

            // display screen with pipes and bird
            display.clear();
            display.show(&mut timer, screen, 200);
            screen[bird_coord.0][bird_coord.1] = 0;
            display.show(&mut timer, screen, 100);
            screen[bird_coord.0][bird_coord.1] = 1;
            display.show(&mut timer, screen, 200);

            // hit detection
            // use pipe number to determine if bird is not in the gap
            // note pipe value of 1 corresponds to bird coord equal to 4
            // and pipe value of 5 corresponds to bird coord equal to 0
            if pipes[1] != 0 {
                if (bird_coord.0 !=  5 - pipes[1]) && (bird_coord.0 != 4 - pipes[1]) {
                    game_over = true;
                }
            }
            
            // increment counter to indicate pipes have moved over one space
            counter += 1;

            timer.delay_ms(500 as u16);
        }

        //rprintln!("Game is over")
        cortex_m::interrupt::free(move |cs| {
            *SPEAKER_OFF.borrow(cs).borrow_mut() = true;
        });
    }
}

// RTC interrupt, exectued for each RTC tick
#[interrupt]
fn RTC0() {
    static mut COUNTER: u32 = 0;
    static mut SWITCHES: [u32; 3] = [5, 8, 20];
    static mut STATE: usize = 0;
    /* Enter critical section */
    cortex_m::interrupt::free(|cs| {
        /* Borrow devices */
        if let (Some(speaker), Some(rtc)) = (
            SPEAKER.borrow(cs).borrow().as_ref(),
            RTC.borrow(cs).borrow().as_ref(),
        ) {
            if *STATE == 0 {
                speaker.set_prescaler(pwm::Prescaler::Div128);
                speaker.set_period(Hertz(2));
                let max_duty = speaker.max_duty();
                speaker.set_duty_on_common(max_duty / 4);
            } else if *STATE == 1 {
                speaker.set_prescaler(pwm::Prescaler::Div128);
                speaker.set_period(Hertz(80));
                let max_duty = speaker.max_duty();
                speaker.set_duty_on_common(max_duty / 4);
            } else if *STATE == 2 {
                speaker.set_prescaler(pwm::Prescaler::Div64);
                speaker.set_period(Hertz(400));
                let max_duty = speaker.max_duty();
                speaker.set_duty_on_common(max_duty / 8);
            }

            // Restart the PWM at 50% duty cycle
            // let max_duty = speaker.max_duty();
            // speaker.set_duty_on_common(max_duty / 4);

            if SPEAKER_OFF.borrow(cs).clone().into_inner() {
                speaker.disable();
                rtc.disable_counter();
            }

            // Clear the RTC interrupt
            rtc.reset_event(RtcInterrupt::Tick);
        }
    });

    *COUNTER += 1;
    if *COUNTER >= SWITCHES[*STATE] {
        *COUNTER = 0;
        *STATE = (*STATE + 1) % 3;
    }
}