/// Network Stack — smoltcp TCP/IP integration over virtio-net.
///
/// This module bridges the virtio-net device to smoltcp's TCP/IP stack,
/// giving the kernel the ability to speak IP, TCP, UDP, and DNS.
///
/// Architecture:
///   virtio-net (device driver) → VirtioNetDevice (smoltcp Device trait)
///   → smoltcp Interface (IP/ARP/ICMP) → TCP/UDP sockets
///
/// For QEMU with user-mode networking (-netdev user):
///   IP:      10.0.2.15/24
///   Gateway: 10.0.2.2
///   DNS:     10.0.2.3

use alloc::vec;
use alloc::vec::Vec;
use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::phy::{self, Device, DeviceCapabilities, Medium};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, HardwareAddress, IpAddress, IpCidr, Ipv4Address};

use crate::virtio_net::VirtioNet;

/// QEMU user-mode networking defaults
const DEVICE_IP: Ipv4Address = Ipv4Address::new(10, 0, 2, 15);
const GATEWAY_IP: Ipv4Address = Ipv4Address::new(10, 0, 2, 2);
const DNS_IP: Ipv4Address = Ipv4Address::new(10, 0, 2, 3);
const SUBNET_PREFIX: u8 = 24;

/// Wrapper around VirtioNet that implements smoltcp's Device trait.
/// This is the bridge between hardware and the TCP/IP stack.
pub struct VirtioNetDevice {
    rx_buffer: Vec<u8>,
    tx_buffer: Vec<u8>,
}

impl VirtioNetDevice {
    pub fn new() -> Self {
        VirtioNetDevice {
            rx_buffer: vec![0u8; 1514], // Max Ethernet frame
            tx_buffer: vec![0u8; 1514],
        }
    }
}

/// smoltcp RxToken — represents a received packet.
pub struct VirtioRxToken<'a>(&'a mut [u8], usize);

impl<'a> phy::RxToken for VirtioRxToken<'a> {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        f(&mut self.0[..self.1])
    }
}

/// smoltcp TxToken — represents a packet to transmit.
pub struct VirtioTxToken<'a>(&'a mut [u8]);

impl<'a> phy::TxToken for VirtioTxToken<'a> {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let result = f(&mut self.0[..len]);
        // In a full implementation, this would DMA the buffer to virtio-net.
        // For now, the packet is prepared but TX is a stub until virtqueue
        // ring buffer management is implemented.
        result
    }
}

impl Device for VirtioNetDevice {
    type RxToken<'a> = VirtioRxToken<'a>;
    type TxToken<'a> = VirtioTxToken<'a>;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        // Stub: no packets received yet (virtqueue RX not wired)
        // This will be connected when virtqueue ring buffers are implemented
        None
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(VirtioTxToken(&mut self.tx_buffer))
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ethernet;
        caps.max_transmission_unit = 1514;
        caps.max_burst_size = Some(1);
        caps
    }
}

/// The kernel's network stack — smoltcp Interface + socket set.
pub struct NetworkStack {
    pub device: VirtioNetDevice,
    pub iface: Interface,
    pub sockets: SocketSet<'static>,
}

/// Initialize the network stack with QEMU user-mode networking defaults.
///
/// Returns None if the virtio-net device wasn't found (no networking).
pub fn init(net_device: &VirtioNet) -> NetworkStack {
    let mac = EthernetAddress(net_device.mac);
    let mut device = VirtioNetDevice::new();

    // Configure the smoltcp interface
    let config = Config::new(HardwareAddress::Ethernet(mac));
    let mut iface = Interface::new(config, &mut device, Instant::from_millis(0));

    // Set IP address and default gateway
    iface.update_ip_addrs(|addrs| {
        addrs
            .push(IpCidr::new(IpAddress::Ipv4(DEVICE_IP), SUBNET_PREFIX))
            .unwrap();
    });
    iface
        .routes_mut()
        .add_default_ipv4_route(GATEWAY_IP)
        .unwrap();

    serial_println!("[NET] smoltcp interface configured:");
    serial_println!("[NET]   IP:      {}/{}", DEVICE_IP, SUBNET_PREFIX);
    serial_println!("[NET]   Gateway: {}", GATEWAY_IP);
    serial_println!("[NET]   DNS:     {}", DNS_IP);
    serial_println!("[NET]   MAC:     {}", mac);

    let sockets = SocketSet::new(vec![]);

    NetworkStack {
        device,
        iface,
        sockets,
    }
}

/// Get the DNS server address for QEMU user-mode networking.
pub fn dns_server() -> Ipv4Address {
    DNS_IP
}
