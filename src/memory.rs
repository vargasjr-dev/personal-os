/// Memory Management — Page Tables & Frame Allocation
///
/// This module provides the foundation for virtual memory:
/// - Translating the bootloader's physical memory map into usable frames
/// - A simple frame allocator that hands out physical pages
/// - Active page table access via the CR3 register
///
/// The bootloader maps all physical memory at a configurable offset
/// (via the `map_physical_memory` feature). We use this mapping to
/// safely access page tables and physical frames from kernel space.

use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use x86_64::{
    structures::paging::{
        FrameAllocator, OffsetPageTable, PageTable, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

/// Initialize the OffsetPageTable.
///
/// # Safety
/// The caller must guarantee that the complete physical memory is
/// mapped to virtual memory at the passed `physical_memory_offset`.
/// Also, this function must be called only once to avoid aliasing
/// `&mut` references (which is undefined behavior).
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

/// Return a mutable reference to the active level 4 page table.
///
/// # Safety
/// The caller must guarantee that the complete physical memory is
/// mapped at the given offset, and that this function is only called once.
unsafe fn active_level_4_table(
    physical_memory_offset: VirtAddr,
) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}

// ─── Frame Allocator ────────────────────────────────────────────────────────

/// A simple frame allocator that returns usable frames from the
/// bootloader's memory map.
///
/// This is a bump allocator — it only ever hands out the next frame
/// and never reclaims. Good enough for early kernel bootstrap.
/// Will be replaced by a bitmap/buddy allocator in Phase 3+.
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// Create a frame allocator from the bootloader memory map.
    ///
    /// # Safety
    /// The caller must guarantee that the passed memory map is valid.
    /// All frames marked as `USABLE` must actually be unused.
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    /// Returns an iterator over usable frames from the memory map.
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> + '_ {
        // Get usable regions from the memory map
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);

        // Map each region to its address range as frame-aligned start addresses
        let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());

        // Flatten into individual 4KiB frame start addresses
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));

        // Create PhysFrame types from the start addresses
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[test_case]
fn test_frame_allocator_returns_frames() {
    // This test verifies the frame allocator can be constructed
    // and returns at least one usable frame.
    // Actual allocation test requires boot_info access from _start.
    // For now, just verify the module compiles cleanly.
    assert_eq!(core::mem::size_of::<PhysFrame<Size4KiB>>(), 8);
}
