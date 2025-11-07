use switchr::{ConfigBuilder, FrameTypes, PortConfig, Switch, SwitchrError, VlanConfig, VlanId};
use switchr::tap::TapHardware;

fn main() -> Result<(), SwitchrError> {
    let config = ConfigBuilder::new()
        .with_ports(vec![
            PortConfig::new(FrameTypes::Untagged, VlanId::ONE),
            PortConfig::new(FrameTypes::Untagged, VlanId::ONE),
            PortConfig::new(FrameTypes::All, VlanId::ONE),
            PortConfig::new(FrameTypes::Untagged, VlanId::ONE),
        ])
        .with_vlan(VlanId::ONE, VlanConfig::new(vec![0,1,2,3], vec![]))
        .with_vlan(VlanId::new(3003).unwrap(), VlanConfig::new(vec![], vec![]))
        .build()?;

    Switch::new(config, TapHardware::new(4)?).run()
}
