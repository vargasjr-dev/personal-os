/// CPU Exception & Hardware Interrupt Handlers
///
/// This module initializes the Interrupt Descriptor Table (IDT),
/// the 8259 PIC (Programmable Interrupt Controller), and registers
/// handlers for CPU exceptions and hardware interrupts.
///
/// Hardware interrupts covered:
/// - Timer (PIC line 0) — fires ~18.2 Hz, drives preemptive scheduling
/// - Keyboard (PIC line 1) — PS/2 scancode → key event translation
///
/// References: Intel SDM Vol. 3A, Chapter 6; OSDev PIC wiki

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin::Mutex;

// ─── PIC Configuration ──────────────────────────────────────────────────────

/// Offset for the primary PIC (IRQ 0-7 → interrupt 32-39).
/// Must not overlap with CPU exceptions (0-31).
pub const PIC_1_OFFSET: u8 = 32;

/// Offset for the secondary PIC (IRQ 8-15 → interrupt 40-47).
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

/// The chained 8259 PIC pair, protected by a spinlock for safe access.
pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

/// Hardware interrupt vector numbers
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard = PIC_1_OFFSET + 1,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

// ─── IDT Setup ──────────────────────────────────────────────────────────────

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // Exception #3: Breakpoint (INT3)
        idt.breakpoint.set_handler_fn(breakpoint_handler);

        // Exception #8: Double Fault (dedicated stack)
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(DOUBLE_FAULT_IST_INDEX);
        }

        // Hardware: Timer (IRQ 0)
        idt[InterruptIndex::Timer.as_usize()]
            .set_handler_fn(timer_interrupt_handler);

        // Hardware: Keyboard (IRQ 1)
        idt[InterruptIndex::Keyboard.as_usize()]
            .set_handler_fn(keyboard_interrupt_handler);

        idt
    };
}

/// Load the IDT into the CPU
pub fn init_idt() {
    IDT.load();
}

// ─── Exception Handlers ─────────────────────────────────────────────────────

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    crate::serial_println!("[EXCEPTION] Breakpoint\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    crate::serial_println!("[EXCEPTION] DOUBLE FAULT\n{:#?}", stack_frame);
    panic!("DOUBLE FAULT — cannot recover");
}

// ─── Hardware Interrupt Handlers ────────────────────────────────────────────

/// Timer interrupt (IRQ 0, ~18.2 Hz).
/// Currently just acknowledges the interrupt. Will drive scheduling later.
extern "x86-interrupt" fn timer_interrupt_handler(
    _stack_frame: InterruptStackFrame,
) {
    // Acknowledge the interrupt so the PIC continues delivering
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

/// Keyboard interrupt (IRQ 1).
/// Reads the PS/2 scancode from port 0x60 and translates to key events.
extern "x86-interrupt" fn keyboard_interrupt_handler(
    _stack_frame: InterruptStackFrame,
) {
    use x86_64::instructions::port::Port;
    use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};

    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            Mutex::new(Keyboard::new(
                ScancodeSet1::new(),
                layouts::Us104Key,
                HandleControl::Ignore,
            ));
    }

    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => {
                    crate::serial_print!("{}", character);
                    crate::print!("{}", character);
                }
                DecodedKey::RawKey(key) => {
                    crate::serial_print!("{:?}", key);
                }
            }
        }
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

// ─── GDT & TSS for Double Fault Stack ───────────────────────────────────────

use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor, SegmentSelector};
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5; // 20 KiB
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            stack_start + STACK_SIZE
        };
        tss
    };
}

lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (gdt, Selectors { code_selector, tss_selector })
    };
}

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

pub fn init_gdt() {
    use x86_64::instructions::tables::load_tss;
    use x86_64::instructions::segmentation::{CS, Segment};

    GDT.0.load();
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
}

/// Initialize all interrupt handling: GDT → TSS → IDT → PIC → enable interrupts
pub fn init() {
    init_gdt();
    init_idt();

    // Initialize the 8259 PIC and unmask all interrupt lines
    unsafe { PICS.lock().initialize() };

    // Enable hardware interrupts (clear the interrupt-disable flag)
    x86_64::instructions::interrupts::enable();
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[test_case]
fn test_breakpoint_exception() {
    x86_64::instructions::interrupts::int3();
}
