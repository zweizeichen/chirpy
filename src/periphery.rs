use minifb::{Key, Window, WindowOptions};
use rodio::{source::SineWave, Sink};

// Screen dimensions
pub const SCREEN_WIDTH: u16 = 64;
pub const SCREEN_HEIGHT: u16 = 32;
pub const SCREEN_SIZE: usize = 64 * 32;

// Screen scale
const WINDOW_SCALE: minifb::Scale = minifb::Scale::X16;

// Background color
const BACKGROUND_COLOR: u32 = 0x00_00_00;

// Draw color on screen (RGB)
const DRAW_COLOR: u32 = 0xff_ff_ff;

// Sine beep frequency in Hz
const BEEP_FREQ: u32 = 440;

pub struct Periphery {
    pub framebuffer: [u8; SCREEN_SIZE],
    window: Window,
    audio_sink: Sink,
}

impl Default for Periphery {
    // Create a new empty screen
    fn default() -> Periphery {
        let options = WindowOptions {
            borderless: false,
            resize: false,
            scale: WINDOW_SCALE,
            title: true,
        };

        let window = Window::new(
            "chirpy",
            usize::from(SCREEN_WIDTH),
            usize::from(SCREEN_HEIGHT),
            options,
        )
        .unwrap_or_else(|e| {
            panic!("{}", e);
        });

        let audio_device = rodio::default_output_device().unwrap_or_else(|| {
            panic!("Unable to initialize default audio device!");
        });

        let audio_sink = Sink::new(&audio_device);
        audio_sink.pause();
        audio_sink.append(SineWave::new(BEEP_FREQ));

        Periphery {
            framebuffer: [0; SCREEN_SIZE],
            window,
            audio_sink,
        }
    }
}

impl Periphery {
    // Draw contents of framebuffer to display
    pub fn draw_screen(&mut self) {
        if self.window.is_open() {
            let mut buffer_32bits: [u32; SCREEN_SIZE] = [BACKGROUND_COLOR; SCREEN_SIZE];

            for (pixel_index, pixel) in self.framebuffer.iter().enumerate() {
                if *pixel > 0 {
                    // Convert non-zero values to draw color on screen
                    buffer_32bits[pixel_index] = DRAW_COLOR;
                }
            }

            self.window.update_with_buffer(&buffer_32bits).unwrap();
        }
    }

    // Get currently pressed key code as per key map, otherwise 0xff
    pub fn get_current_key_code(&mut self) -> u8 {
        let mut key_code: u8 = 0xff;
        let keys_option = self.window.get_keys();

        if keys_option.is_some() {
            let keys = keys_option.unwrap();

            if !keys.is_empty() {
                let key = keys[0];

                key_code = match key {
                    Key::X => 0x0,
                    Key::Key1 => 0x1,
                    Key::Key2 => 0x2,
                    Key::Key3 => 0x3,
                    Key::Q => 0x4,
                    Key::W => 0x5,
                    Key::E => 0x6,
                    Key::A => 0x7,
                    Key::S => 0x8,
                    Key::D => 0x9,
                    Key::Z => 0xA,
                    Key::C => 0xB,
                    Key::Key4 => 0xC,
                    Key::R => 0xD,
                    Key::F => 0xE,
                    Key::V => 0xF,
                    _ => 0xff,
                };
            }
        }

        key_code
    }

    // Start playing sound
    pub fn play_sound(&mut self) {
        self.audio_sink.play();
    }

    // Stop playing sound
    pub fn stop_sound(&mut self) {
        self.audio_sink.pause();
    }
}
