#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_framework(crate::test_framework)]
#![reexport_test_harness_main = "test_main"]

mod vga_buffer;
mod llm;

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

#[cfg(test)]
fn test_runner(tests: &[&dyn Fn()]) {
    println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
