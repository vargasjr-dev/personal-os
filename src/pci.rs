/// PCI Bus Enumeration — discovers devices on the PCI bus.
///
/// Uses x86 I/O ports 0xCF8 (config address) and 0xCFC (config data)
/// to enumerate all PCI devices. This is how the kernel finds the
/// virtio-net device that QEMU exposes.

use x86_64::instructions::port::Port;
use alloc::vec::Vec;

/// A PCI device identified by bus, device, function.
#[derive(Debug, Clone, Copy)]
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_code: u8,
    pub subclass: u8,
    pub header_type: u8,
}

impl PciDevice {
    /// Read a 32-bit value from PCI configuration space.
    pub fn config_read(&self, offset: u8) -> u32 {
        let address: u32 = (1 << 31)
            | ((self.bus as u32) << 16)
            | ((self.device as u32) << 11)
            | ((self.function as u32) << 8)
            | ((offset as u32) & 0xFC);

        unsafe {
            let mut addr_port = Port::<u32>::new(0xCF8);
            let mut data_port = Port::<u32>::new(0xCFC);
            addr_port.write(address);
            data_port.read()
        }
    }

    /// Read a BAR (Base Address Register) value.
    pub fn bar(&self, index: u8) -> u32 {
        self.config_read(0x10 + (index * 4))
    }

    /// Check if this is a virtio device (vendor 0x1AF4).
    pub fn is_virtio(&self) -> bool {
        self.vendor_id == 0x1AF4
    }

    /// Check if this is a virtio-net device (device 0x1000 or 0x1041).
    pub fn is_virtio_net(&self) -> bool {
        self.is_virtio() && (self.device_id == 0x1000 || self.device_id == 0x1041)
    }
}

/// Read a PCI config dword at (bus, device, function, offset).
fn pci_config_read(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let address: u32 = (1 << 31)
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | ((offset as u32) & 0xFC);

    unsafe {
        let mut addr_port = Port::<u32>::new(0xCF8);
        let mut data_port = Port::<u32>::new(0xCFC);
        addr_port.write(address);
        data_port.read()
    }
}

/// Enumerate all PCI devices on the bus.
/// Scans bus 0 (sufficient for QEMU's default config).
pub fn enumerate() -> Vec<PciDevice> {
    let mut devices = Vec::new();

    for bus in 0..=255u8 {
        for device in 0..32u8 {
            let vendor_device = pci_config_read(bus, device, 0, 0);
            let vendor_id = (vendor_device & 0xFFFF) as u16;

            if vendor_id == 0xFFFF {
                continue; // No device
            }

            let device_id = ((vendor_device >> 16) & 0xFFFF) as u16;
            let class_rev = pci_config_read(bus, device, 0, 0x08);
            let class_code = ((class_rev >> 24) & 0xFF) as u8;
            let subclass = ((class_rev >> 16) & 0xFF) as u8;
            let header = pci_config_read(bus, device, 0, 0x0C);
            let header_type = ((header >> 16) & 0xFF) as u8;

            devices.push(PciDevice {
                bus,
                device,
                function: 0,
                vendor_id,
                device_id,
                class_code,
                subclass,
                header_type,
            });

            // Check multi-function devices
            if header_type & 0x80 != 0 {
                for function in 1..8u8 {
                    let vd = pci_config_read(bus, device, function, 0);
                    let vid = (vd & 0xFFFF) as u16;
                    if vid == 0xFFFF {
                        continue;
                    }
                    let did = ((vd >> 16) & 0xFFFF) as u16;
                    let cr = pci_config_read(bus, device, function, 0x08);
                    let cc = ((cr >> 24) & 0xFF) as u8;
                    let sc = ((cr >> 16) & 0xFF) as u8;
                    let h = pci_config_read(bus, device, function, 0x0C);
                    let ht = ((h >> 16) & 0xFF) as u8;

                    devices.push(PciDevice {
                        bus,
                        device,
                        function,
                        vendor_id: vid,
                        device_id: did,
                        class_code: cc,
                        subclass: sc,
                        header_type: ht,
                    });
                }
            }
        }

        // Only scan bus 0 for QEMU (fast path)
        if bus == 0 {
            break;
        }
    }

    devices
}

/// Find the first virtio-net device on the PCI bus.
pub fn find_virtio_net() -> Option<PciDevice> {
    enumerate().into_iter().find(|d| d.is_virtio_net())
}
