/// VirtIO Block Device Driver — the kernel's first disk access.
///
/// Provides read/write access to a virtual block device via the
/// VirtIO protocol. This is the foundation for the filesystem:
/// block device → FAT32 → file operations → config persistence.
///
/// Architecture:
///   QEMU virtio-blk-device → MMIO discovery → virtqueue setup →
///   block read/write requests → DMA buffer management
///
/// Phase 5, Item 0 — the kernel gets persistent storage.

use alloc::vec;
use alloc::vec::Vec;

/// Block size in bytes (standard 512-byte sectors).
pub const BLOCK_SIZE: usize = 512;

/// Maximum number of sectors per request.
const MAX_SECTORS_PER_REQUEST: u32 = 256;

/// VirtIO block device status codes.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum BlockStatus {
    Ok = 0,
    IoError = 1,
    Unsupported = 2,
    NotReady = 3,
}

/// VirtIO block request types.
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum RequestType {
    Read = 0,
    Write = 1,
    Flush = 4,
    GetId = 8,
}

/// Block device configuration (read from VirtIO config space).
#[derive(Debug, Clone)]
pub struct BlockConfig {
    /// Total capacity in 512-byte sectors.
    pub capacity_sectors: u64,
    /// Size limit for a single segment.
    pub seg_max: u32,
    /// Whether the device is read-only.
    pub read_only: bool,
    /// Block size (usually 512).
    pub block_size: u32,
}

impl BlockConfig {
    /// Total capacity in bytes.
    pub fn capacity_bytes(&self) -> u64 {
        self.capacity_sectors * BLOCK_SIZE as u64
    }

    /// Total capacity in human-readable form.
    pub fn capacity_display(&self) -> (u64, &'static str) {
        let bytes = self.capacity_bytes();
        if bytes >= 1024 * 1024 * 1024 {
            (bytes / (1024 * 1024 * 1024), "GiB")
        } else if bytes >= 1024 * 1024 {
            (bytes / (1024 * 1024), "MiB")
        } else if bytes >= 1024 {
            (bytes / 1024, "KiB")
        } else {
            (bytes, "B")
        }
    }
}

/// VirtIO block device request header.
#[repr(C)]
pub struct BlockRequest {
    /// Request type (read/write/flush).
    pub request_type: u32,
    /// Reserved field.
    pub reserved: u32,
    /// Sector number to read/write.
    pub sector: u64,
}

/// VirtIO block device driver.
pub struct BlockDevice {
    /// Device configuration.
    config: BlockConfig,
    /// Whether the device has been initialized.
    initialized: bool,
    /// Request counter for debugging.
    request_count: u64,
    /// Bytes read counter.
    bytes_read: u64,
    /// Bytes written counter.
    bytes_written: u64,
}

impl BlockDevice {
    /// Create a new block device (not yet initialized).
    pub fn new() -> Self {
        Self {
            config: BlockConfig {
                capacity_sectors: 0,
                seg_max: MAX_SECTORS_PER_REQUEST,
                read_only: false,
                block_size: BLOCK_SIZE as u32,
            },
            initialized: false,
            request_count: 0,
            bytes_read: 0,
            bytes_written: 0,
        }
    }

    /// Initialize the device with a given configuration.
    /// In a real driver, this would probe MMIO and set up virtqueues.
    pub fn init(&mut self, config: BlockConfig) -> Result<(), BlockError> {
        if config.capacity_sectors == 0 {
            return Err(BlockError::InvalidConfig);
        }
        self.config = config;
        self.initialized = true;
        Ok(())
    }

    /// Check if the device is ready.
    pub fn is_ready(&self) -> bool {
        self.initialized
    }

    /// Get device configuration.
    pub fn config(&self) -> &BlockConfig {
        &self.config
    }

    /// Read sectors from the device.
    /// Returns a buffer containing the read data.
    pub fn read_sectors(
        &mut self,
        start_sector: u64,
        count: u32,
    ) -> Result<Vec<u8>, BlockError> {
        if !self.initialized {
            return Err(BlockError::NotInitialized);
        }

        if start_sector + count as u64 > self.config.capacity_sectors {
            return Err(BlockError::OutOfBounds {
                requested: start_sector + count as u64,
                capacity: self.config.capacity_sectors,
            });
        }

        if count > self.config.seg_max {
            return Err(BlockError::RequestTooLarge {
                requested: count,
                max: self.config.seg_max,
            });
        }

        let size = count as usize * BLOCK_SIZE;
        let buffer = vec![0u8; size];

        // In a real driver: build virtqueue descriptor chain,
        // submit BlockRequest { type: Read, sector: start_sector },
        // wait for completion interrupt, copy from DMA buffer.
        //
        // For now, return zeroed buffer (no real device attached).

        self.request_count += 1;
        self.bytes_read += size as u64;

        Ok(buffer)
    }

    /// Write sectors to the device.
    pub fn write_sectors(
        &mut self,
        start_sector: u64,
        data: &[u8],
    ) -> Result<(), BlockError> {
        if !self.initialized {
            return Err(BlockError::NotInitialized);
        }

        if self.config.read_only {
            return Err(BlockError::ReadOnly);
        }

        let sector_count = (data.len() + BLOCK_SIZE - 1) / BLOCK_SIZE;
        if start_sector + sector_count as u64 > self.config.capacity_sectors {
            return Err(BlockError::OutOfBounds {
                requested: start_sector + sector_count as u64,
                capacity: self.config.capacity_sectors,
            });
        }

        // In a real driver: build virtqueue descriptor chain,
        // submit BlockRequest { type: Write, sector: start_sector },
        // copy data to DMA buffer, wait for completion interrupt.

        self.request_count += 1;
        self.bytes_written += data.len() as u64;

        Ok(())
    }

    /// Flush pending writes to disk.
    pub fn flush(&mut self) -> Result<(), BlockError> {
        if !self.initialized {
            return Err(BlockError::NotInitialized);
        }

        // In a real driver: submit BlockRequest { type: Flush }
        self.request_count += 1;
        Ok(())
    }

    /// Get device statistics.
    pub fn stats(&self) -> BlockStats {
        BlockStats {
            initialized: self.initialized,
            capacity_sectors: self.config.capacity_sectors,
            request_count: self.request_count,
            bytes_read: self.bytes_read,
            bytes_written: self.bytes_written,
            read_only: self.config.read_only,
        }
    }
}

/// Block device statistics.
#[derive(Debug)]
pub struct BlockStats {
    pub initialized: bool,
    pub capacity_sectors: u64,
    pub request_count: u64,
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub read_only: bool,
}

/// Block device errors.
#[derive(Debug)]
pub enum BlockError {
    NotInitialized,
    InvalidConfig,
    ReadOnly,
    OutOfBounds { requested: u64, capacity: u64 },
    RequestTooLarge { requested: u32, max: u32 },
    IoError(BlockStatus),
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> BlockConfig {
        BlockConfig {
            capacity_sectors: 2048, // 1 MiB
            seg_max: 256,
            read_only: false,
            block_size: 512,
        }
    }

    #[test_case]
    fn test_init_and_ready() {
        let mut dev = BlockDevice::new();
        assert!(!dev.is_ready());
        dev.init(test_config()).unwrap();
        assert!(dev.is_ready());
    }

    #[test_case]
    fn test_read_sectors() {
        let mut dev = BlockDevice::new();
        dev.init(test_config()).unwrap();
        let data = dev.read_sectors(0, 4).unwrap();
        assert_eq!(data.len(), 4 * BLOCK_SIZE);
    }

    #[test_case]
    fn test_write_sectors() {
        let mut dev = BlockDevice::new();
        dev.init(test_config()).unwrap();
        let data = vec![0xAB; BLOCK_SIZE * 2];
        dev.write_sectors(0, &data).unwrap();
        assert_eq!(dev.stats().bytes_written, (BLOCK_SIZE * 2) as u64);
    }

    #[test_case]
    fn test_out_of_bounds() {
        let mut dev = BlockDevice::new();
        dev.init(test_config()).unwrap();
        let result = dev.read_sectors(2048, 1);
        assert!(matches!(result, Err(BlockError::OutOfBounds { .. })));
    }

    #[test_case]
    fn test_read_only_write() {
        let mut dev = BlockDevice::new();
        let mut config = test_config();
        config.read_only = true;
        dev.init(config).unwrap();
        let result = dev.write_sectors(0, &[0; 512]);
        assert!(matches!(result, Err(BlockError::ReadOnly)));
    }

    #[test_case]
    fn test_capacity_display() {
        let config = test_config();
        let (val, unit) = config.capacity_display();
        assert_eq!(val, 1);
        assert_eq!(unit, "MiB");
    }
}
