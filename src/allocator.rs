/// Heap Allocator — Maps a virtual memory region and plugs in a linked-list allocator.
///
/// This gives the kernel access to Rust's `alloc` crate: Vec, Box, String,
/// Arc, and everything else that needs dynamic memory.
///
/// The heap lives at a fixed virtual address range (HEAP_START..HEAP_START+HEAP_SIZE).
/// We map this range to physical frames using the page mapper from memory.rs,
/// then hand the region to a linked-list allocator wrapped in a spinlock.

use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};
use linked_list_allocator::LockedHeap;

/// Start address of the kernel heap (virtual).
/// Chosen to avoid collisions with other mapped regions.
pub const HEAP_START: usize = 0x_4444_4444_0000;

/// Size of the kernel heap: 100 KiB.
/// Enough for early kernel bootstrap. Can be grown later by mapping more pages.
pub const HEAP_SIZE: usize = 100 * 1024;

/// The global allocator used by Rust's `alloc` crate.
/// LockedHeap wraps a linked-list allocator in a spinlock for interrupt safety.
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Initialize the kernel heap.
///
/// 1. Maps HEAP_SIZE bytes of virtual memory starting at HEAP_START
///    to physical frames provided by the frame allocator.
/// 2. Initializes the linked-list allocator with the mapped region.
///
/// Must be called after memory::init() provides a working page mapper
/// and frame allocator.
pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    // Map each page in the heap range to a fresh physical frame
    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?.flush();
        }
    }

    // Initialize the allocator with the mapped region
    unsafe {
        ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE);
    }

    Ok(())
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_heap_constants() {
        // Heap start must be page-aligned (4 KiB)
        assert_eq!(HEAP_START % 4096, 0);
        // Heap size must be positive and page-aligned
        assert!(HEAP_SIZE > 0);
        assert_eq!(HEAP_SIZE % 4096, 0);
    }
}
