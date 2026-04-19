/// Async Keyboard Input — bridges hardware interrupts to the async executor.
///
/// The keyboard IRQ handler pushes scancodes into a fixed-size ring buffer.
/// The async `KeyStream` reads from this buffer, yielding decoded characters
/// one at a time. This is the kernel's first real I/O bridge:
/// hardware interrupt → buffer → async task → user interaction.

use alloc::string::String;
use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use futures_util::{stream::Stream, task::AtomicWaker};
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};

/// Global scancode queue — filled by the keyboard IRQ, drained by async tasks.
/// 100 scancodes is plenty of buffer for human typing speed.
static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();

/// Waker to notify the async executor when a new scancode arrives.
static WAKER: AtomicWaker = AtomicWaker::new();

/// Called by the keyboard interrupt handler to enqueue a scancode.
/// Must be safe to call from interrupt context (no allocation, no blocking).
pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if queue.push(scancode).is_err() {
            serial_println!("[WARN] Keyboard queue full — scancode dropped");
        } else {
            WAKER.wake();
        }
    } else {
        serial_println!("[WARN] Keyboard queue not initialized");
    }
}

/// Async stream of decoded keyboard characters.
pub struct KeyStream {
    keyboard: Keyboard<layouts::Us104Key, ScancodeSet1>,
}

impl KeyStream {
    /// Create a new KeyStream. Initializes the global scancode queue on first call.
    pub fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("KeyStream::new should only be called once");

        KeyStream {
            keyboard: Keyboard::new(
                ScancodeSet1::new(),
                layouts::Us104Key,
                HandleControl::Ignore,
            ),
        }
    }
}

impl Stream for KeyStream {
    type Item = char;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<char>> {
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("scancode queue not initialized");

        // Try to decode scancodes into characters
        loop {
            match queue.pop() {
                Some(scancode) => {
                    if let Ok(Some(key_event)) = self.keyboard.add_byte(scancode) {
                        if let Some(key) = self.keyboard.process_keyevent(key_event) {
                            match key {
                                DecodedKey::Unicode(c) => return Poll::Ready(Some(c)),
                                DecodedKey::RawKey(_) => continue, // Skip raw keys
                            }
                        }
                    }
                }
                None => {
                    // Queue empty — register waker and return Pending
                    WAKER.register(cx.waker());
                    // Double-check after registering (avoid race condition)
                    match queue.pop() {
                        Some(scancode) => {
                            WAKER.take();
                            if let Ok(Some(key_event)) = self.keyboard.add_byte(scancode) {
                                if let Some(key) = self.keyboard.process_keyevent(key_event) {
                                    match key {
                                        DecodedKey::Unicode(c) => {
                                            return Poll::Ready(Some(c))
                                        }
                                        DecodedKey::RawKey(_) => continue,
                                    }
                                }
                            }
                        }
                        None => return Poll::Pending,
                    }
                }
            }
        }
    }
}

/// Async input loop — reads characters and builds a line buffer.
/// On Enter, prints the line back as a command echo.
/// This is the kernel's first interactive prompt.
pub async fn input_loop() {
    use futures_util::StreamExt;

    let mut keys = KeyStream::new();
    let mut line = String::new();

    crate::println!("> ");

    while let Some(c) = keys.next().await {
        match c {
            '\n' | '\r' => {
                crate::println!();
                if !line.is_empty() {
                    crate::println!("[echo] {}", line);
                    serial_println!("[INPUT] {}", line);
                }
                line.clear();
                crate::print!("> ");
            }
            '\u{8}' => {
                // Backspace
                if line.pop().is_some() {
                    crate::print!("\u{8} \u{8}");
                }
            }
            c if !c.is_control() => {
                line.push(c);
                crate::print!("{}", c);
            }
            _ => {} // Ignore other control characters
        }
    }
}
