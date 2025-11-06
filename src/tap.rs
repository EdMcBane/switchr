use std::io::Error;
use tappers::{DeviceState, Interface, Tap};
use crate::{Hardware, PortNumber, SwitchrError};

pub struct TapHardware(Vec<Tap>);

impl TapHardware {
    pub fn new(num_ports: usize) -> Result<TapHardware, SwitchrError> {
        let taps = (0..num_ports).map(|i| {
            let if_name = Interface::new(format!("switchr{}", i))?;
            let mut tap = Tap::new_named(if_name)?;
            tap.set_state(DeviceState::Up)?;
            Ok(tap)
        }).collect::<Result<Vec<_>, SwitchrError>>()?;
        Ok(TapHardware(taps))
    }
}

impl Hardware for TapHardware {
    fn send(&mut self, port_number: PortNumber, data: &[u8]) -> Result<(), SwitchrError> {
        self.0[port_number].send(data)?;
        Ok(())
    }

    fn recv(&mut self) -> Result<(PortNumber, Vec<u8>), SwitchrError> {
        for port in self.0.iter_mut() {
            port.recv()
        }
    }
}

impl From<std::io::Error> for SwitchrError {
    fn from(value: Error) -> Self {
        SwitchrError::Io(value)
    }
}