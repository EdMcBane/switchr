use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;
use moka::policy::EvictionPolicy;

// https://en.wikipedia.org/wiki/IEEE_802.1Q
// TODO: immagine diagramma con DBra

fn main() {
    // TODO: validate config
    // TODO: create builder
    // TODO: "config" module
    let config = Config {
        ports: vec![
            PortConfig { acceptable_frame_types: AcceptableFrameTypes::Untagged, pvid: VlanId::ONE },
            PortConfig { acceptable_frame_types: AcceptableFrameTypes::Untagged, pvid: VlanId::ONE },
            PortConfig { acceptable_frame_types: AcceptableFrameTypes::All, pvid: VlanId::ONE },
            PortConfig { acceptable_frame_types: AcceptableFrameTypes::Untagged, pvid: VlanId::ONE },
        ],
        vlans: [
            (VlanId::ONE, VlanConfig::new(vec![0,1,2,3], vec![])),
            (VlanId::new(3003).unwrap(), VlanConfig::new(vec![], vec![])),
        ].into_iter().collect()
    };

    Switch::new(4, config).run();
}

struct Switch {
    ports: usize, // TODO: use to validate configuration
    ingress: Ingress,
    egress: Egress,
    fowarding_table: Rc<RefCell<FowardingTable>>,
}

impl Switch {
    fn new(ports: usize, config: Config) -> Switch {
        let config = Rc::new(config);
        let forwarding_table = Rc::new(RefCell::new(FowardingTable::new(65536)));
        Switch {
            ports,
            ingress: Ingress(config.clone()),
            egress: Egress { config, forwarding_table: forwarding_table.clone() },
            fowarding_table: forwarding_table,
        }
    }

    fn run(&mut self) -> ! {
        loop {
            let (port_num, buf) = Self::recv();
            if let Some(frame) = self.ingress.process(port_num, buf) {
                self.fowarding_table.borrow_mut().update(frame.vlan_id, frame.frame.src, port_num);
                self.egress.dispatch(frame);
            }
        }
    }

    fn recv() -> (PortNumber, Vec<u8>) {
        todo!()
    }
}

pub struct Config {
    pub ports: Vec<PortConfig>,
    pub vlans: HashMap<VlanId, VlanConfig>,
}

pub struct PortConfig {
    acceptable_frame_types: AcceptableFrameTypes,
    pvid: VlanId
}

pub struct VlanConfig {
    untagged_ports: Vec<usize>,
    tagged_ports: Vec<usize>,
}

impl PortConfig {
    pub fn new(acceptable_frame_types: AcceptableFrameTypes, pvid: VlanId) -> PortConfig {
        PortConfig {
            acceptable_frame_types,
            pvid,
        }
    }
}

impl VlanConfig {
    pub fn new(untagged_ports: Vec<usize>, tagged_ports: Vec<usize>) -> VlanConfig {
        VlanConfig {
            untagged_ports,
            tagged_ports,
        }
    }
}

pub enum AcceptableFrameTypes {
    Tagged,
    Untagged,
    All,
}

impl AcceptableFrameTypes {
    pub(crate) fn accepts(&self, dot1q: &Option<VlanId>) -> bool {
        match (self, dot1q) {
            (AcceptableFrameTypes::All, _) => true,
            (AcceptableFrameTypes::Tagged, Some(_)) => true,
            (AcceptableFrameTypes::Untagged, None) => true,
            _ => false,
        }
    }
}

struct Ingress(Rc<Config>);

const ETH_TYPE_DOT1Q: u16 = 0x8100;

impl Ingress {
    pub fn process(&self, port_number: PortNumber, data: Vec<u8>) -> Option<ScopedFrame> {
        let port_config = &self.0.ports[port_number];
        let (dot1q, frame) = Self::parse_frame(data).ok()?;

        if !port_config.acceptable_frame_types.accepts(&dot1q) {
            return None;
        }
        Some(ScopedFrame {
            vlan_id: dot1q.unwrap_or(port_config.pvid),
            frame,
        })
    }

    pub fn parse_frame(data: Vec<u8>) -> Result<(Option<VlanId>, Frame), ParseError> { // TODO: return result
        // TODO: check parsing
        let dst = data[0..6].try_into().unwrap();
        let src = data[6..12].try_into().unwrap();
        let eth_type = u16::from_be_bytes([data[12], data[13]]);
        let (dot1q, rest) = if eth_type == ETH_TYPE_DOT1Q {
            let vid = u16::from_be_bytes([data[14], data[15]]) & 0xFFF;
            (Some(VlanId(vid)), data[16..].to_vec())
        } else {
            (None, data[12..].to_vec())
        };
        Ok((dot1q, Frame {
            dst,
            src,
            rest,
        }))
    }
}

#[derive(Debug)]
struct ParseError;

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ParseError")
    }
}

impl Error for ParseError {}

struct ScopedFrame {
    vlan_id: VlanId,
    frame: Frame,
}

impl ScopedFrame {
    pub(crate) fn tagged(&self) -> Vec<u8> {
        let mut result: Vec<u8> = Vec::new();
        result.extend(&self.frame.dst);
        result.extend(&self.frame.src);
        result.extend(&[0x81, 0x00]);
        result.extend(self.vlan_id.to_u16().to_be_bytes());
        result.extend(&self.frame.rest);
        result
    }
}

impl ScopedFrame {
    pub(crate) fn untagged(&self) -> Vec<u8> {
        let mut result: Vec<u8> = Vec::new();
        result.extend(&self.frame.dst);
        result.extend(&self.frame.src);
        result.extend(&self.frame.rest);
        result
    }
}

struct Frame {
    dst: HwAddr,
    src: HwAddr,
    rest: Vec<u8>,
}

type HwAddr = [u8; 6];

struct Egress {
    config: Rc<Config>,
    forwarding_table: Rc<RefCell<FowardingTable>>,
}

impl Egress {
    fn dispatch(&self, frame: ScopedFrame) {
        if let Some(vlan_config) = self.config.vlans.get(&frame.vlan_id) {
            let known_port =  self.forwarding_table.borrow()
                .lookup(frame.vlan_id, frame.frame.dst);

            let untagged = frame.untagged();
            for &port_number in vlan_config.untagged_ports.iter()
                .filter(|&&p| known_port.is_none() || p == known_port.unwrap()) {
                Self::send(port_number, &untagged);
            }

            let tagged = frame.tagged();
            for &port_number in vlan_config.tagged_ports.iter().filter(|&&p| known_port.is_none() || p == known_port.unwrap()) {
                Self::send(port_number, &tagged);
            }
        }
    }

    fn send(_port_number: PortNumber, _data: &[u8]) {
        todo!()
    }
}
type PortNumber = usize;

// Potential zero-val optimization using NonZeroU16
#[derive(Debug, Eq, Hash, PartialEq, Clone, Copy)]
pub struct VlanId(u16);


impl VlanId {
    const ONE: VlanId = VlanId(1);

    pub fn new(vlan_id: u16) -> Option<Self> {
        Some(vlan_id)
            .filter(|&vlan_id| vlan_id > 0 && vlan_id < 4096)
            .map(|v| Self(v))
    }

    fn to_u16(&self) -> u16 {
        self.0
    }
}


#[derive(Default)]
struct FowardingTable {
    size: u64,
    caches: HashMap<VlanId, moka::sync::Cache<HwAddr, PortNumber>>
}

impl FowardingTable {

    fn new(size: u64) -> Self {
        Self {
            size,
            caches: HashMap::new()
        }
    }

    fn lookup(&self, vlan_id: VlanId, mac: HwAddr) -> Option<PortNumber> {
        self.caches.get(&vlan_id)?.get(&mac)
    }

    fn update(&mut self, vlan_id: VlanId, mac: HwAddr, port_number: PortNumber) {
        self.caches.entry(vlan_id).or_insert_with(|| {
            moka::sync::CacheBuilder::new(self.size)
                .eviction_policy(EvictionPolicy::lru())
                .build()
        }).insert(mac, port_number);
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use crate::{Ingress, VlanId};

    #[test]
    fn parse_eth_ip() -> Result<(), Box<dyn Error>> {
        let packet = hex::decode("003018051cc074563cffd2780800")?;
        let (dot1q, frame) = Ingress::parse(packet);
        assert_eq!(dot1q, None);
        assert_eq!(frame.dst, [0x00, 0x30, 0x18, 0x05, 0x1c, 0xc0]);
        assert_eq!(frame.src, [0x74, 0x56, 0x3c, 0xff, 0xd2, 0x78]);
        assert_eq!(frame.rest[0..2], [0x8, 0x00]);
        Ok(())
    }

    #[test]
    fn parse_eth_dot1q_ip() -> Result<(), Box<dyn Error>> {
        let packet = hex::decode("001562643341001c582364c18100000a0800")?;
        let (dot1q, frame) = Ingress::parse(packet);
        assert_eq!(dot1q, Some(VlanId(10)));
        assert_eq!(frame.dst, [0x00, 0x15, 0x62, 0x64, 0x33, 0x41]);
        assert_eq!(frame.src, [0x00, 0x1c, 0x58, 0x23, 0x64, 0xc1]);
        assert_eq!(frame.rest[0..2], [0x8, 0x00]);
        Ok(())
    }
}