#![no_main]
#![no_std]

use cortex_m_rt::entry;
use embedded_hal::{digital::v2::InputPin, blocking::delay::DelayMs};
use rtt_target::{rtt_init_print, rprintln};
use panic_rtt_target as _;

use microbit::{
    display::blocking::Display,
    hal::{Timer, Rng},
};

#[derive(Debug)]
pub enum Pipe {
    Low,
    MidLow,
    HighLow,
    High,
}

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let board = microbit::Board::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);
    let mut display = Display::new(board.display_pins);

    let button_a = board.buttons.button_a;
    let button_b = board.buttons.button_b;

    let mut rng = Rng::new(board.RNG);
    
    let mut bird_coord = (2, 1);
    let mut game_over = false;
    
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
                    4 => game_over = true,
                    _ => bird_coord.0 = bird_coord.0 + 1,
                }
                
                // bird falls one step
                screen[bird_coord.0][bird_coord.1] = 1;
                //rprintln!("bird fell");

                // timer.delay_ms(500 as u16);
            }

            // display screen with pipes and bird
            display.clear();
            display.show(&mut timer, screen, 500);

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
    }
}
