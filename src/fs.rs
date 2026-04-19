/// FAT32 Filesystem — the kernel's file abstraction layer.
///
/// Provides a filesystem interface on top of the VirtIO block device.
/// Uses FAT32 for broad compatibility — QEMU disk images, USB drives,
/// and SD cards all speak FAT32 natively.
///
/// Architecture:
///   BlockDevice → FAT32 partition → directories + files
///   FileSystem::open() → FileHandle → read/write/seek
///
/// Phase 5, Item 1 — the kernel can organize persistent data.

use alloc::string::String;
use alloc::vec::Vec;

use crate::block::{BlockDevice, BlockError, BLOCK_SIZE};

/// FAT32 Boot sector fields (offsets into sector 0).
const FAT32_SIGNATURE: u16 = 0xAA55;
const BYTES_PER_SECTOR_OFFSET: usize = 11;
const SECTORS_PER_CLUSTER_OFFSET: usize = 13;
const RESERVED_SECTORS_OFFSET: usize = 14;
const NUM_FATS_OFFSET: usize = 16;
const SECTORS_PER_FAT_OFFSET: usize = 36;
const ROOT_CLUSTER_OFFSET: usize = 44;

/// Maximum filename length (8.3 format).
const MAX_FILENAME: usize = 11;

/// FAT32 directory entry size.
const DIR_ENTRY_SIZE: usize = 32;

/// File attributes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FileAttributes(u8);

impl FileAttributes {
    pub const READ_ONLY: Self = Self(0x01);
    pub const HIDDEN: Self = Self(0x02);
    pub const SYSTEM: Self = Self(0x04);
    pub const DIRECTORY: Self = Self(0x10);
    pub const ARCHIVE: Self = Self(0x20);

    pub fn is_directory(self) -> bool {
        self.0 & Self::DIRECTORY.0 != 0
    }

    pub fn is_read_only(self) -> bool {
        self.0 & Self::READ_ONLY.0 != 0
    }

    pub fn is_hidden(self) -> bool {
        self.0 & Self::HIDDEN.0 != 0
    }
}

/// A directory entry parsed from FAT32.
#[derive(Debug, Clone)]
pub struct DirEntry {
    /// Short filename (8.3 format, trimmed).
    pub name: String,
    /// File attributes.
    pub attributes: FileAttributes,
    /// First cluster of file data.
    pub first_cluster: u32,
    /// File size in bytes.
    pub size: u32,
}

/// FAT32 filesystem parameters (parsed from boot sector).
#[derive(Debug, Clone)]
pub struct FatParams {
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub num_fats: u8,
    pub sectors_per_fat: u32,
    pub root_cluster: u32,
}

impl FatParams {
    /// Calculate the first sector of the data region.
    pub fn data_start_sector(&self) -> u64 {
        self.reserved_sectors as u64
            + (self.num_fats as u64 * self.sectors_per_fat as u64)
    }

    /// Convert a cluster number to its first sector.
    pub fn cluster_to_sector(&self, cluster: u32) -> u64 {
        self.data_start_sector()
            + ((cluster - 2) as u64 * self.sectors_per_cluster as u64)
    }
}

/// FAT32 filesystem.
pub struct FileSystem {
    /// Underlying block device.
    device: BlockDevice,
    /// Parsed FAT32 parameters.
    params: Option<FatParams>,
    /// Whether the filesystem is mounted.
    mounted: bool,
    /// Files opened counter.
    files_opened: u64,
}

impl FileSystem {
    /// Create a new filesystem (not yet mounted).
    pub fn new(device: BlockDevice) -> Self {
        Self {
            device,
            params: None,
            mounted: false,
            files_opened: 0,
        }
    }

    /// Mount the filesystem by reading and validating the boot sector.
    pub fn mount(&mut self) -> Result<(), FsError> {
        if !self.device.is_ready() {
            return Err(FsError::DeviceNotReady);
        }

        // Read boot sector (sector 0)
        let boot_sector = self.device.read_sectors(0, 1)
            .map_err(|e| FsError::BlockError(e))?;

        // Validate boot signature
        let sig = u16::from_le_bytes([boot_sector[510], boot_sector[511]]);
        if sig != FAT32_SIGNATURE {
            return Err(FsError::InvalidSignature(sig));
        }

        // Parse FAT32 parameters
        let params = FatParams {
            bytes_per_sector: u16::from_le_bytes([
                boot_sector[BYTES_PER_SECTOR_OFFSET],
                boot_sector[BYTES_PER_SECTOR_OFFSET + 1],
            ]),
            sectors_per_cluster: boot_sector[SECTORS_PER_CLUSTER_OFFSET],
            reserved_sectors: u16::from_le_bytes([
                boot_sector[RESERVED_SECTORS_OFFSET],
                boot_sector[RESERVED_SECTORS_OFFSET + 1],
            ]),
            num_fats: boot_sector[NUM_FATS_OFFSET],
            sectors_per_fat: u32::from_le_bytes([
                boot_sector[SECTORS_PER_FAT_OFFSET],
                boot_sector[SECTORS_PER_FAT_OFFSET + 1],
                boot_sector[SECTORS_PER_FAT_OFFSET + 2],
                boot_sector[SECTORS_PER_FAT_OFFSET + 3],
            ]),
            root_cluster: u32::from_le_bytes([
                boot_sector[ROOT_CLUSTER_OFFSET],
                boot_sector[ROOT_CLUSTER_OFFSET + 1],
                boot_sector[ROOT_CLUSTER_OFFSET + 2],
                boot_sector[ROOT_CLUSTER_OFFSET + 3],
            ]),
        };

        // Basic validation
        if params.bytes_per_sector == 0 || params.sectors_per_cluster == 0 {
            return Err(FsError::InvalidParams);
        }

        self.params = Some(params);
        self.mounted = true;
        Ok(())
    }

    /// Check if mounted.
    pub fn is_mounted(&self) -> bool {
        self.mounted
    }

    /// Get filesystem parameters.
    pub fn params(&self) -> Option<&FatParams> {
        self.params.as_ref()
    }

    /// List entries in the root directory.
    pub fn list_root(&mut self) -> Result<Vec<DirEntry>, FsError> {
        let params = self.params.as_ref().ok_or(FsError::NotMounted)?;
        let sector = params.cluster_to_sector(params.root_cluster);
        let sectors_to_read = params.sectors_per_cluster as u32;

        let data = self.device.read_sectors(sector, sectors_to_read)
            .map_err(|e| FsError::BlockError(e))?;

        let mut entries = Vec::new();
        for chunk in data.chunks(DIR_ENTRY_SIZE) {
            if chunk.len() < DIR_ENTRY_SIZE {
                break;
            }

            // End of directory
            if chunk[0] == 0x00 {
                break;
            }

            // Deleted entry
            if chunk[0] == 0xE5 {
                continue;
            }

            // Parse 8.3 filename
            let name_bytes = &chunk[..MAX_FILENAME];
            let name = core::str::from_utf8(name_bytes)
                .unwrap_or("?")
                .trim()
                .to_string();

            let attributes = FileAttributes(chunk[11]);
            let first_cluster = u32::from_le_bytes([
                chunk[26], chunk[27], chunk[20], chunk[21],
            ]);
            let size = u32::from_le_bytes([
                chunk[28], chunk[29], chunk[30], chunk[31],
            ]);

            entries.push(DirEntry {
                name,
                attributes,
                first_cluster,
                size,
            });
        }

        Ok(entries)
    }

    /// Get filesystem stats.
    pub fn stats(&self) -> FsStats {
        FsStats {
            mounted: self.mounted,
            files_opened: self.files_opened,
            has_params: self.params.is_some(),
        }
    }
}

/// Filesystem statistics.
#[derive(Debug)]
pub struct FsStats {
    pub mounted: bool,
    pub files_opened: u64,
    pub has_params: bool,
}

/// Filesystem errors.
#[derive(Debug)]
pub enum FsError {
    DeviceNotReady,
    NotMounted,
    InvalidSignature(u16),
    InvalidParams,
    BlockError(BlockError),
    FileNotFound(String),
    IsDirectory,
    ReadOnly,
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::BlockConfig;

    fn mock_device() -> BlockDevice {
        let mut dev = BlockDevice::new();
        dev.init(BlockConfig {
            capacity_sectors: 2048,
            seg_max: 256,
            read_only: false,
            block_size: 512,
        }).unwrap();
        dev
    }

    #[test_case]
    fn test_fs_not_mounted() {
        let dev = mock_device();
        let fs = FileSystem::new(dev);
        assert!(!fs.is_mounted());
        assert!(fs.params().is_none());
    }

    #[test_case]
    fn test_mount_invalid_signature() {
        let dev = mock_device();
        let mut fs = FileSystem::new(dev);
        // Device returns zeroed sectors — signature will be 0x0000
        let result = fs.mount();
        assert!(matches!(result, Err(FsError::InvalidSignature(0))));
    }

    #[test_case]
    fn test_fat_params_calculations() {
        let params = FatParams {
            bytes_per_sector: 512,
            sectors_per_cluster: 8,
            reserved_sectors: 32,
            num_fats: 2,
            sectors_per_fat: 1024,
            root_cluster: 2,
        };

        // data_start = 32 + (2 * 1024) = 2080
        assert_eq!(params.data_start_sector(), 2080);
        // cluster 2 → sector 2080 + (2-2)*8 = 2080
        assert_eq!(params.cluster_to_sector(2), 2080);
        // cluster 3 → sector 2080 + (3-2)*8 = 2088
        assert_eq!(params.cluster_to_sector(3), 2088);
    }

    #[test_case]
    fn test_file_attributes() {
        let dir = FileAttributes::DIRECTORY;
        assert!(dir.is_directory());
        assert!(!dir.is_read_only());

        let ro = FileAttributes::READ_ONLY;
        assert!(ro.is_read_only());
        assert!(!ro.is_directory());
    }
}
