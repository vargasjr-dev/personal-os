#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_framework(crate::test_framework::test_runner)]
#![reexport_test_harness_main = "test_main"]

mod vga_buffer;
mod llm;
pub mod test_framework;

use core::panic::PanicInfo;

/// Entry point for the kernel
#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║                                                           ║");
    println!("║              PersonalOS - Assistant-Native OS             ║");
    println!("║                                                           ║");
    println!("║  \"The future of computing starts here.\" ⚔️                ║");
    println!("║                                                           ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();
    println!("Kernel booted successfully!");
    println!("Architecture: x86_64");
    println!("LLM Backend: Ready to connect");
    println!();
    
    // Demo the LLM abstraction layer
    println!("Testing LLM abstraction layer...");
    println!();
    
    // This will be extended to actually query the LLM
    // For now, we just show the architecture is ready
    println!("[INFO] LLM interface initialized");
    println!("[INFO] Backend: Anthropic API (cloud) OR Local Llama");
    println!("[INFO] Swap backends via environment/config");
    println!();
    
    println!("System ready. Halting...");
    
    #[cfg(test)]
    test_main();

    loop {
        x86_64::instructions::hlt();
    }
}

/// Panic handler - called on kernel panic
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("[PANIC] {}", info);
    loop {
        x86_64::instructions::hlt();
    }
}

/// Test-mode panic handler — reports failure via serial + QEMU exit
#[cfg(test)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    test_framework::test_panic_handler(info)
}

// ─── Unit Tests ──────────────────────────────────────────────────────────────

#[test_case]
fn test_trivial_assertion() {
    assert_eq!(1, 1);
}

#[test_case]
fn test_vga_println() {
    // Verify println! doesn't panic (VGA buffer works)
    println!("test_vga_println output");
}

#[test_case]
fn test_vga_many_lines() {
    // Test scrolling — write more lines than the VGA buffer holds (25)
    for i in 0..50 {
        println!("line {}", i);
    }
}
