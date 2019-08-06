use crate::bin::*;
use crate::periphery::{Periphery, SCREEN_HEIGHT, SCREEN_SIZE, SCREEN_WIDTH};

use std::convert::TryInto;
use std::ops::Add;
use std::thread::sleep;
use std::time::{Duration, Instant};

use rand::Rng;
use std::ops::Sub;

const MEMORY_SIZE: usize = 4_096;
const TARGET_FPS: u32 = 60;
const CPU_CLOCK_IN_HZ: u32 = 1_000;

const CYCLES_PER_FRAME: u32 = CPU_CLOCK_IN_HZ / TARGET_FPS;
const TIMER_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 60);
const FRAME_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / TARGET_FPS as u64);

const FONTSET_OFFSET: u16 = 0x50;

pub struct System {
    program_counter: usize,
    memory: [u8; MEMORY_SIZE],

    stack: [usize; 25],
    stack_pointer: usize,

    v_registers: [u8; 16],
    index_register: u16,

    delay_timer: u8,
    sound_timer: u8,

    // Strictly speaking this would be a 'u4'
    keyboard_input: u8,

    // Helper structures for simulation
    cycles_in_current_frame: u32,
    next_frame_tick: Instant,
    next_timer_tick: Instant,

    // Peripherials
    periphery: Periphery,
}

impl Default for System {
    // Initialize system state, load bitfont and set program counter to 0x200 as per convention
    fn default() -> System {
        let fontset: [u8; 80] = [
            0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80, // F
        ];

        let mut system = System {
            program_counter: 0x200,
            memory: [0; MEMORY_SIZE],

            stack: [0; 25],
            stack_pointer: 0,

            v_registers: [0; 16],
            index_register: 0,

            delay_timer: 0,
            sound_timer: 0,

            keyboard_input: 0,

            next_timer_tick: Instant::now(),
            next_frame_tick: Instant::now(),
            cycles_in_current_frame: 0,
            periphery: Periphery::default(),
        };

        // Copy fontset with offset
        let mut position: usize = usize::from(FONTSET_OFFSET);
        for data in fontset.iter() {
            system.memory[position] = *data;
            position += 1;
        }

        system
    }
}

impl System {
    // Load data
    pub fn copy_buffer_to_memory(&mut self, buffer: Vec<u8>, offset: usize) {
        if buffer.len() + offset <= MEMORY_SIZE {
            let mut counter = offset;
            for data in buffer {
                self.memory[counter] = data;
                counter += 1;
            }
        } else {
            panic!("You tried to load a data file which does not fit into memory!")
        }
    }

    // Enter main run loop (blocks)
    pub fn run(&mut self) {
        loop {
            // Limit maximum number of cycles per frame
            if self.cycles_in_current_frame < CYCLES_PER_FRAME {
                self.cycle();
                self.cycles_in_current_frame += 1;
            } else {
                self.get_input();
                self.tick_frame();
                self.tick_timers();
                self.sleep_if_needed();
            }
        }
    }

    // Execute cycle
    #[allow(clippy::cognitive_complexity)]
    fn cycle(&mut self) {
        // Get current op code
        let upper = u16::from(self.memory[self.program_counter]) << 8;
        let lower = u16::from(self.memory[self.program_counter + 1]);
        let opcode: u16 = upper | lower;

        // Register macros
        macro_rules! second_nibble_register {
            () => {
                self.v_registers[to_usize(second_nibble(opcode))]
            };
        }

        macro_rules! third_nibble_register {
            () => {
                self.v_registers[to_usize(third_nibble(opcode))]
            };
        }

        // The big opcode matcher
        match first_nibble(opcode) {
            0x0 => match opcode {
                0xE0 => {
                    // Clear screen
                    self.periphery.framebuffer = [0; SCREEN_SIZE];
                    self.program_counter += 2;
                }
                0xEE => {
                    // Return from subroutine
                    self.program_counter = self.stack[self.stack_pointer];
                    self.stack_pointer -= 1;
                }
                _ => {
                    // Call program in lower three nibbles, ignored
                    self.program_counter += 2;
                }
            },
            0x1 => {
                // Jump to lower three nibbles
                self.program_counter = to_usize(lower_three(opcode));
            }
            0x2 => {
                // Call subroutine at lower three nibbles
                self.stack_pointer += 1;
                self.stack[self.stack_pointer] = self.program_counter + 2;
                self.program_counter = to_usize(lower_three(opcode));
            }
            0x3 => {
                // Skip next instruction if second nibble register equals lower half
                let equals: bool = second_nibble_register!() == to_byte(lower_half(opcode));

                if equals {
                    self.program_counter += 4;
                } else {
                    self.program_counter += 2;
                }
            }
            0x4 => {
                // Skip next instruction if second nibble register does not equal lower half
                let equals: bool = second_nibble_register!() == to_byte(lower_half(opcode));

                if !equals {
                    self.program_counter += 4;
                } else {
                    self.program_counter += 2;
                }
            }
            0x5 => match fourth_nibble(opcode) {
                0x0 => {
                    // Skip next instruction if second nibble register equals third nibble register
                    let equals: bool = second_nibble_register!() == third_nibble_register!();

                    if equals {
                        self.program_counter += 4;
                    } else {
                        self.program_counter += 2;
                    }
                }
                _ => self.panic_unknown_opcode(opcode),
            },
            0x6 => {
                // Set second nibble register to lower half
                second_nibble_register!() = to_byte(lower_half(opcode));
                self.program_counter += 2;
            }
            0x7 => {
                // Adds lower half to second nibble register (does not affect carry flag)
                second_nibble_register!() =
                    second_nibble_register!().wrapping_add(to_byte(lower_half(opcode)));
                self.program_counter += 2;
            }
            0x8 => match fourth_nibble(opcode) {
                0x0 => {
                    // Set second nibble register to third nibble register
                    second_nibble_register!() = third_nibble_register!();
                    self.program_counter += 2;
                }
                0x1 => {
                    // OR second nibble register with third nibble register
                    second_nibble_register!() =
                        second_nibble_register!() | third_nibble_register!();
                    self.program_counter += 2;
                }
                0x2 => {
                    // AND second nibble register with third nibble register
                    second_nibble_register!() =
                        second_nibble_register!() & third_nibble_register!();
                    self.program_counter += 2;
                }
                0x3 => {
                    // XOR second nibble register with third nibble register
                    second_nibble_register!() =
                        second_nibble_register!() ^ third_nibble_register!();
                    self.program_counter += 2;
                }
                0x4 => {
                    // Add third nibble register to second nibble register, set carry
                    let (result, wrapped) =
                        second_nibble_register!().overflowing_add(third_nibble_register!());
                    self.v_registers[15] = if wrapped { 1 } else { 0 };
                    second_nibble_register!() = result;
                    self.program_counter += 2;
                }
                0x5 => {
                    // Subtract third nibble register from second nibble register, set borrow
                    let (result, wrapped) =
                        second_nibble_register!().overflowing_sub(third_nibble_register!());
                    self.v_registers[15] = if wrapped { 0 } else { 1 };
                    second_nibble_register!() = result;
                    self.program_counter += 2;
                }
                0x6 => {
                    // Take LSB of second nibble register and store in carry/borrow, shift register right by 1
                    let register_value = second_nibble_register!();
                    self.v_registers[15] = register_value & 0x0001;
                    second_nibble_register!() >>= 1;
                    self.program_counter += 2;
                }
                0x7 => {
                    // Set second nibble register to (third nibble register - second nibble register), set borrow
                    let (result, wrapped) =
                        third_nibble_register!().overflowing_sub(second_nibble_register!());
                    self.v_registers[15] = if wrapped { 0 } else { 1 };
                    second_nibble_register!() = result;
                    self.program_counter += 2;
                }
                0xE => {
                    // Take MSB of second nibble register and store in carry/borrow, shift register left by 1
                    let register_value = second_nibble_register!();
                    self.v_registers[15] = (register_value & 0b1000_0000) >> 7;
                    second_nibble_register!() <<= 1;
                    self.program_counter += 2;
                }
                _ => self.panic_unknown_opcode(opcode),
            },
            0x9 => match fourth_nibble(opcode) {
                0x0 => {
                    // Skip next instruction if second nibble register does not equal third nibble register
                    let equals: bool = second_nibble_register!() == third_nibble_register!();

                    if !equals {
                        self.program_counter += 4;
                    } else {
                        self.program_counter += 2;
                    }
                }
                _ => self.panic_unknown_opcode(opcode),
            },
            0xA => {
                // Set index register to lower three nibbles
                self.index_register = lower_three(opcode);
                self.program_counter += 2;
            }
            0xB => {
                // Jump to lower three nibbles plus first register
                self.program_counter =
                    to_usize(lower_three(opcode)) + to_usize(u16::from(self.v_registers[0]));
            }
            0xC => {
                // Set second nibble register to random byte ANDed with lower half
                second_nibble_register!() =
                    rand::thread_rng().gen::<u8>() & to_byte(lower_half(opcode));
                self.program_counter += 2;
            }
            0xD => {
                // Draw sprite with height of fourth nibble at (second nibble register, third nibble register)
                // if any pixel gets hidden, set carry/borrow
                let height = fourth_nibble(opcode);
                let top_x = u16::from(second_nibble_register!());
                let top_y = u16::from(third_nibble_register!());

                let mut hidden: bool = false;

                for y_index in 0..height {
                    let bitmap = self.memory[usize::from(self.index_register + y_index)];
                    for x_index in 0..8 {
                        let y = (top_y + y_index) % SCREEN_HEIGHT;
                        let x = (top_x + (7 - x_index)) % SCREEN_WIDTH;
                        let framebuffer_index = usize::from(y * SCREEN_WIDTH + x);
                        let pixel_value = (bitmap >> x_index) & 0x1;
                        let new_value = pixel_value ^ self.periphery.framebuffer[framebuffer_index];

                        if !hidden
                            && new_value == 0
                            && self.periphery.framebuffer[framebuffer_index] != 0
                        {
                            hidden = true;
                        }

                        self.periphery.framebuffer[framebuffer_index] = new_value;
                    }
                }

                self.v_registers[15] = if hidden { 1 } else { 0 };
                self.program_counter += 2;
            }
            0xE => match lower_half(opcode) {
                0x9E => {
                    // Skip next instruction if key at second nibble register is pressed
                    if self.keyboard_input == second_nibble_register!() {
                        self.program_counter += 4;
                    } else {
                        self.program_counter += 2;
                    }
                }
                0xA1 => {
                    // Skip next instruction if key at second nibble register is not pressed
                    if self.keyboard_input != second_nibble_register!() {
                        self.program_counter += 4;
                    } else {
                        self.program_counter += 2;
                    }
                }
                _ => self.panic_unknown_opcode(opcode),
            },
            0xF => match lower_half(opcode) {
                0x07 => {
                    // Set second nibble register to delay timer's value
                    second_nibble_register!() = self.delay_timer;
                    self.program_counter += 2;
                }
                0x0A => {
                    // Block until key-press, store result in second nibble register
                    if self.keyboard_input != 0xff {
                        second_nibble_register!() = self.keyboard_input;
                        self.program_counter += 2;
                    }
                }
                0x15 => {
                    // Set delay timer to second nibble register
                    self.delay_timer = second_nibble_register!();
                    self.program_counter += 2;
                }
                0x18 => {
                    // Set sound timer to second nibble register
                    self.sound_timer = second_nibble_register!();
                    if self.sound_timer > 0 {
                        self.periphery.play_sound();
                    }

                    self.program_counter += 2;
                }
                0x1E => {
                    // Add second nibble register to index register
                    self.index_register = self
                        .index_register
                        .wrapping_add(u16::from(second_nibble_register!()));
                    self.program_counter += 2;
                }
                0x29 => {
                    // Set index register to character sprite address determined by second nibble register
                    self.index_register = u16::from(second_nibble_register!()) * 5 + FONTSET_OFFSET;
                    self.program_counter += 2;
                }
                0x33 => {
                    // Store BCD of second nibble register
                    // Hundreds at index register
                    // Tens at index register plus one
                    // Ones at index register plus two

                    // Well, let's just use a string for now :P
                    // Yes I know there are more efficient ways but I don't want to copy.

                    let mut number_string = second_nibble_register!().to_string();

                    for i in 0..3 {
                        let address = usize::from(self.index_register + i);
                        self.memory[address] = number_string
                            .pop()
                            .unwrap_or('0')
                            .to_digit(10)
                            .unwrap()
                            .try_into()
                            .unwrap();
                    }

                    self.program_counter += 2;
                }
                0x55 => {
                    // Store registers from first register to second nibble register (inclusive) starting at the address of the index register
                    let upper_bound = second_nibble(opcode) + 1;
                    for i in 0..upper_bound {
                        let address = usize::from(self.index_register + i);
                        self.memory[address] = self.v_registers[usize::from(i)];
                    }

                    self.program_counter += 2;
                }
                0x65 => {
                    // Populate registers from first register to second nibble register starting from the address stored in the index register
                    let upper_bound = second_nibble(opcode) + 1;
                    for i in 0..upper_bound {
                        let address = usize::from(self.index_register + i);
                        self.v_registers[usize::from(i)] = self.memory[address];
                    }

                    self.program_counter += 2;
                }
                _ => self.panic_unknown_opcode(opcode),
            },
            _ => self.panic_unknown_opcode(opcode),
        }
    }

    // Write key code to input register
    fn get_input(&mut self) {
        self.keyboard_input = self.periphery.get_current_key_code();
    }

    // Tick frame timer
    fn tick_frame(&mut self) {
        let now = Instant::now();

        if self.next_frame_tick <= now {
            self.cycles_in_current_frame = 0;
            self.periphery.draw_screen();
            self.next_frame_tick = now.add(FRAME_INTERVAL);
        }
    }

    // Tick both timers at 60Hz
    fn tick_timers(&mut self) {
        let now = Instant::now();

        if self.next_timer_tick <= now {
            if self.delay_timer != 0 {
                self.delay_timer -= 1;
            }

            if self.sound_timer != 0 {
                self.sound_timer -= 1;
            } else {
                self.periphery.stop_sound();
            }

            self.next_timer_tick = now.add(TIMER_INTERVAL);
        }
    }

    // Sleep if needed (we assume a 1ms accuracy of the sleep timer)
    fn sleep_if_needed(&mut self) {
        let now = Instant::now();

        if now < self.next_frame_tick {
            let until = self.next_frame_tick.sub(now);

            if until > Duration::from_millis(1) {
                sleep(until.sub(Duration::from_millis(1)));
            }
        }
    }

    fn panic_unknown_opcode(&self, opcode: u16) {
        panic!(
            "Unknown opcode: {:#X} at address {:#X}!",
            opcode, self.program_counter
        );
    }
}
