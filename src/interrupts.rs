/// CPU Exception Handlers — IDT setup, breakpoint, and double fault
///
/// This module initializes the Interrupt Descriptor Table (IDT) and
/// registers handlers for critical CPU exceptions. Without this,
/// any exception (division by zero, invalid opcode, page fault)
/// triggers a triple fault → immediate reboot.
///
/// References: Intel SDM Vol. 3A, Chapter 6 (Interrupt and Exception Handling)

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use lazy_static::lazy_static;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // Exception #3: Breakpoint (INT3)
        // Used for debugging — safe to continue after handling
        idt.breakpoint.set_handler_fn(breakpoint_handler);

        // Exception #8: Double Fault
        // Fires when the CPU fails to invoke an exception handler.
        // If this fails too → triple fault → hardware reset.
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(DOUBLE_FAULT_IST_INDEX);
        }

        idt
    };
}

/// Load the IDT into the CPU
pub fn init_idt() {
    IDT.load();
}

// ─── Handler Functions ───────────────────────────────────────────────────────

/// Breakpoint handler — logs the exception and continues execution.
/// This is benign; the CPU can resume after INT3.
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    crate::serial_println!("[EXCEPTION] Breakpoint\n{:#?}", stack_frame);
}

/// Double fault handler — logs and halts. Cannot recover from this.
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    crate::serial_println!("[EXCEPTION] DOUBLE FAULT\n{:#?}", stack_frame);
    panic!("DOUBLE FAULT — cannot recover");
}

// ─── GDT & TSS for Double Fault Stack ────────────────────────────────────────

use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor, SegmentSelector};
use x86_64::VirtAddr;

/// IST index for the double fault handler's dedicated stack
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        // Allocate a small stack for the double fault handler.
        // The double fault handler gets its own stack so it can handle
        // stack overflow (which itself causes a double fault).
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5; // 20 KiB
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            stack_start + STACK_SIZE // Stack grows downward
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

/// Initialize the GDT with a TSS that provides a dedicated double-fault stack.
/// Must be called before init_idt().
pub fn init_gdt() {
    use x86_64::instructions::tables::load_tss;
    use x86_64::instructions::segmentation::{CS, Segment};

    GDT.0.load();
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
}

/// Initialize all CPU exception handling: GDT → TSS → IDT
pub fn init() {
    init_gdt();
    init_idt();
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[test_case]
fn test_breakpoint_exception() {
    // Invoke a breakpoint exception — should not panic
    x86_64::instructions::interrupts::int3();
}
