use switchr::{ConfigBuilder, FrameTypes, PortConfig, Switch, SwitchrError, VlanConfig, VlanId};
use switchr::tap::TapHardware;

fn main() -> Result<(), SwitchrError> {
    let config = ConfigBuilder::new()
        .with_ports(vec![
            PortConfig::new(FrameTypes::Untagged, VlanId::ONE),
            PortConfig::new(FrameTypes::Untagged, 1001.try_into()?),
            PortConfig::new(FrameTypes::All, 1002.try_into()?),
            PortConfig::new(FrameTypes::Untagged, 1003.try_into()?),
        ])
        .with_vlan( VlanId::ONE, VlanConfig::new(vec![], vec![0]))
        .with_vlan(1001.try_into()?, VlanConfig::new(vec![1], vec![0]))
        .with_vlan(1002.try_into()?, VlanConfig::new(vec![2], vec![0]))
        .with_vlan(1003.try_into()?, VlanConfig::new(vec![3], vec![0]))
        .build()?;

    Switch::new(config, TapHardware::new(4)?).run()
}
