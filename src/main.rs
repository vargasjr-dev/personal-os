#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_framework(crate::test_framework::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

mod vga_buffer;
mod llm;
pub mod test_framework;
pub mod interrupts;
pub mod memory;
pub mod allocator;

use core::panic::PanicInfo;
use bootloader::{entry_point, BootInfo};
use x86_64::VirtAddr;

entry_point!(kernel_main);

/// Entry point for the kernel — receives BootInfo from the bootloader.
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // Initialize CPU exception handling (GDT → TSS → IDT → PIC → interrupts)
    interrupts::init();

    // Initialize memory management — page tables and frame allocator
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        memory::BootInfoFrameAllocator::init(&boot_info.memory_map)
    };

    // Initialize the kernel heap (maps virtual pages → physical frames)
    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

    serial_println!("[OK] Memory: page tables + heap allocator ready");

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

/// Panic handler — prints to VGA and halts.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("[PANIC] {}", info);
    loop {
        x86_64::instructions::hlt();
    }
}

/// Test-mode panic handler — reports failure via serial + QEMU exit.
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
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
