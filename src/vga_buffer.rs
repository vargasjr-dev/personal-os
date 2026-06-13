use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    /// Write a raw byte+color directly to a specific cell, bypassing
    /// scroll logic. Used by the framebuffer driver.
    pub fn write_raw_cell(&mut self, row: usize, col: usize, byte: u8, fg: Color, bg: Color) {
        if row < BUFFER_HEIGHT && col < BUFFER_WIDTH {
            self.buffer.chars[row][col].write(ScreenChar {
                ascii_character: byte,
                color_code: ColorCode::new(fg, bg),
            });
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                let color_code = self.color_code;
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.column_position += 1;
            }
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                _ => self.write_byte(0xfe),
            }
        }
    }

    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::LightGreen, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::vga_buffer::_print(format_args!("\n")));
    ($fmt:expr) => ($crate::vga_buffer::_print(format_args!(concat!($fmt, "\n"))));
    ($fmt:expr, $($arg:tt)*) => ($crate::vga_buffer::_print(format_args!(concat!($fmt, "\n"), $($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}

/// Write a raw CP437 byte with explicit fg/bg colors to a specific
/// (row, col) cell — no scroll, no ASCII filtering. For framebuffer use.
pub fn write_raw_cell(row: usize, col: usize, byte: u8, fg: Color, bg: Color) {
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        WRITER.lock().write_raw_cell(row, col, byte, fg, bg);
    });
}

/// Read the raw ScreenChar at (row, col). Used by the framebuffer to
/// merge new pixels into existing cells without clobbering neighbours.
pub fn read_cell_colors(row: usize, col: usize) -> (Color, Color) {
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        let writer = WRITER.lock();
        if row < 25 && col < 80 {
            let sc = writer.buffer.chars[row][col].read();
            let code = sc.color_code.0;
            let fg_u8 = code & 0x0F;
            let bg_u8 = (code >> 4) & 0x07; // 3 bits for bg (no blink)
            // Safety: Color is repr(u8) with values 0–15
            let fg = unsafe { core::mem::transmute::<u8, Color>(fg_u8) };
            let bg = unsafe { core::mem::transmute::<u8, Color>(bg_u8) };
            (fg, bg)
        } else {
            (Color::White, Color::Black)
        }
    })
}
