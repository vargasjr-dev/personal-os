/// Virtio-Net Driver — network device for QEMU's virtio-net-pci.
///
/// This is the kernel's first step toward the internet:
/// PCI discovery → virtio handshake → virtqueue setup → ready for packets.
///
/// The actual packet TX/RX comes in the next PR (smoltcp integration).
/// This PR establishes the device detection and initialization protocol.

use crate::pci::{self, PciDevice};

/// Virtio device status flags (per virtio spec 1.1, §2.1)
#[allow(dead_code)]
mod status {
    pub const ACKNOWLEDGE: u8 = 1;
    pub const DRIVER: u8 = 2;
    pub const DRIVER_OK: u8 = 4;
    pub const FEATURES_OK: u8 = 8;
    pub const DEVICE_NEEDS_RESET: u8 = 64;
    pub const FAILED: u8 = 128;
}

/// Virtio-net device wrapper.
pub struct VirtioNet {
    pub pci_device: PciDevice,
    pub io_base: u16,
    pub mac: [u8; 6],
}

impl VirtioNet {
    /// Detect and initialize the virtio-net device.
    ///
    /// Performs the virtio initialization sequence:
    /// 1. Find device on PCI bus
    /// 2. Read I/O base from BAR0
    /// 3. Reset device
    /// 4. Set ACKNOWLEDGE and DRIVER status
    /// 5. Read MAC address
    ///
    /// Returns None if no virtio-net device is found.
    pub fn init() -> Option<Self> {
        let pci_dev = pci::find_virtio_net()?;

        serial_println!(
            "[NET] Found virtio-net at PCI {:02x}:{:02x}.{} (vendor={:#06x}, device={:#06x})",
            pci_dev.bus,
            pci_dev.device,
            pci_dev.function,
            pci_dev.vendor_id,
            pci_dev.device_id,
        );

        // BAR0 contains the I/O port base address
        let bar0 = pci_dev.bar(0);
        let io_base = (bar0 & 0xFFFC) as u16;

        serial_println!("[NET] I/O base: {:#06x}", io_base);

        // Reset the device (write 0 to status register at offset 18)
        unsafe {
            let mut status_port = x86_64::instructions::port::Port::<u8>::new(io_base + 18);
            status_port.write(0);
        }

        // Set ACKNOWLEDGE status
        unsafe {
            let mut status_port = x86_64::instructions::port::Port::<u8>::new(io_base + 18);
            status_port.write(status::ACKNOWLEDGE);
        }

        // Set DRIVER status
        unsafe {
            let mut status_port = x86_64::instructions::port::Port::<u8>::new(io_base + 18);
            let current = status_port.read();
            status_port.write(current | status::DRIVER);
        }

        // Read MAC address from device-specific config (offset 20+)
        let mut mac = [0u8; 6];
        for i in 0..6 {
            unsafe {
                let port = x86_64::instructions::port::Port::<u8>::new(io_base + 20 + i as u16);
                mac[i] = port.read();
            }
        }

        serial_println!(
            "[NET] MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5],
        );

        serial_println!("[NET] Virtio-net device initialized (driver status set)");

        Some(VirtioNet {
            pci_device: pci_dev,
            io_base,
            mac,
        })
    }
}
