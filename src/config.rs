use std::collections::HashMap;
use crate::error::SwitchrError;

pub struct Config {
    pub ports: Vec<PortConfig>,
    pub vlans: HashMap<VlanId, VlanConfig>,
}

// Potential zero-val optimization using NonZeroU16
#[derive(Debug, Eq, Hash, PartialEq, Clone, Copy)]
pub struct VlanId(u16);

impl VlanId {
    pub const ONE: VlanId = VlanId(1);

    pub fn new(vlan_id: u16) -> Option<Self> {
        Some(vlan_id)
            .filter(|&vlan_id| vlan_id > 0 && vlan_id < 4096)
            .map(|v| Self(v))
    }

    pub fn to_u16(&self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for VlanId {
    type Error = SwitchrError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(SwitchrError::InvalidVlan)
    }
}

pub type PortNumber = usize;


pub struct PortConfig {
    pub(crate) acceptable_frame_types: FrameTypes,
    pub(crate) pvid: VlanId
}

pub struct VlanConfig {
    pub(crate) untagged_ports: Vec<usize>,
    pub(crate) tagged_ports: Vec<usize>,
}

impl PortConfig {
    pub fn new(acceptable_frame_types: FrameTypes, pvid: VlanId) -> PortConfig {
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

pub enum FrameTypes {
    Tagged,
    Untagged,
    All,
}

impl FrameTypes {
    pub(crate) fn accepts(&self, dot1q: &Option<VlanId>) -> bool {
        match (self, dot1q) {
            (FrameTypes::All, _) => true,
            (FrameTypes::Tagged, Some(_)) => true,
            (FrameTypes::Untagged, None) => true,
            _ => false,
        }
    }
}

pub struct ConfigBuilder {
    config: Config,
}


impl ConfigBuilder {
    pub fn new() -> Self {
        ConfigBuilder {
            config: Config {
                ports: Default::default(),
                vlans: Default::default(),
            }
        }
    }

    pub fn with_ports(mut self, ports: Vec<PortConfig>) -> Self {
        self.config.ports = ports;
        self
    }

    pub fn with_vlan(mut self, vlan_id: VlanId, config: VlanConfig) -> Self {
        self.config.vlans.insert(vlan_id, config);
        self
    }

    pub fn build(self) -> Result<Config, SwitchrError> {
        if self.config.ports.len() == 0 {
            return Err(SwitchrError::BadConfig("No ports provided".into()));
        }
        for vlan_conf in self.config.vlans.values() {
            for &port in vlan_conf.tagged_ports.iter().chain(vlan_conf.untagged_ports.iter()) {
                if port >= self.config.ports.len() {
                    return Err(SwitchrError::BadConfig(format!("Invalid port {}", port)));
                }
            }
        }
        Ok(self.config)
    }
}