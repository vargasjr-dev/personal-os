/// Test framework for PersonalOS
///
/// Uses QEMU's isa-debug-exit device for CI-friendly test results.
/// Serial port output for test names/results (visible in cargo test).
///
/// Pattern:
///   - Unit tests: #[test_case] in any module
///   - Integration tests: tests/*.rs (each is a separate kernel binary)

use core::fmt;

/// Exit codes for QEMU's isa-debug-exit device
/// The actual exit code is (value << 1) | 1, so:
///   Success (0x10) → exit code 33 (configured in Cargo.toml)
///   Failure (0x11) → exit code 35
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failure = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;
    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

/// Trait for testable functions — prints name, runs, reports result
pub trait Testable {
    fn run(&self);
}

impl<T: Fn()> Testable for T {
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

/// Test runner — called by the custom test framework
pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

/// Panic handler for test mode — prints error and exits with failure
pub fn test_panic_handler(info: &core::panic::PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failure);
    loop {
        x86_64::instructions::hlt();
    }
}

// ─── Serial Port Macros ──────────────────────────────────────────────────────

/// Minimal serial port writer for test output
pub struct SerialPort;

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        use x86_64::instructions::port::Port;
        for byte in s.bytes() {
            unsafe {
                let mut port = Port::new(0x3F8);
                port.write(byte);
            }
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            let mut serial = $crate::test_framework::SerialPort;
            write!(serial, $($arg)*).unwrap();
        }
    };
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => {
        $crate::serial_print!($($arg)*);
        $crate::serial_print!("\n");
    };
}
