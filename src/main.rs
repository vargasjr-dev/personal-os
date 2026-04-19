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
pub mod task;
pub mod keyboard;
pub mod pci;
pub mod virtio_net;
pub mod net;
pub mod dns;
pub mod tls;
pub mod http;
pub mod json;
pub mod anthropic;
pub mod secrets;
pub mod streaming;
pub mod shell;
pub mod context;
pub mod block;

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
    println!("║              PersonalOS - Assistant-Native OS             ║");
    println!("║  \"The future of computing starts here.\" ⚔️                ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();
    println!("Kernel booted. Architecture: x86_64");
    println!();

    // Probe for network device and initialize TCP/IP stack
    match virtio_net::VirtioNet::init() {
        Some(net_dev) => {
            serial_println!("[OK] Network: virtio-net ready");
            let stack = net::init(&net_dev);
            serial_println!("[OK] Network: smoltcp TCP/IP stack ready");
            println!("Network: 10.0.2.15/24 via virtio-net");
            // Stack is initialized — will be used by HTTP client in Phase 3
            core::mem::forget(stack); // Keep alive (static lifetime workaround)
        }
        None => {
            serial_println!("[WARN] No virtio-net device found");
            println!("Network: not available");
        }
    }
    println!();

    // Initialize the async executor and run boot tasks
    serial_println!("[OK] Async executor ready");

    let mut executor = task::simple_executor::SimpleExecutor::new();
    executor.spawn(task::Task::new(boot_message()));
    executor.spawn(task::Task::new(keyboard::input_loop()));
    executor.run();

    #[cfg(test)]
    test_main();

    loop {
        x86_64::instructions::hlt();
    }
}

/// Boot message — first async task, then yields to the input loop.
async fn boot_message() {
    serial_println!("[OK] All boot tasks complete — input loop active");
    println!("System ready. Type something:");
    println!();
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
