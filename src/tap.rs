use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::Receiver;
use tokio_tun::Tun;
use crate::{Hardware, PortNumber, SwitchrError};

pub struct TapHardware {
    rt: Runtime,
    rx: Receiver<Result<(PortNumber, Vec<u8>), SwitchrError>>,
    taps: Vec<Arc<Tun>>,
}


impl TapHardware {
    pub fn new(num_ports: usize) -> Result<TapHardware, SwitchrError> {
        let runtime = tokio::runtime::Runtime::new()?;
        let (tx, rx) = tokio::sync::mpsc::channel(1024);
        let taps = runtime.block_on(async {
            let taps = (0..num_ports).map(|i| {
                let tap = Arc::new(Tun::builder()
                    .tap()
                    .name(&format!("switchr{}", i))
                    .up()
                    .build()?.into_iter().next().unwrap());
                runtime.spawn({
                    let tx = tx.clone();
                    let tap = tap.clone();
                    async move {
                        loop {
                            let mut buffer = vec![0; 2048];
                            let result = tap.recv(&mut buffer).await
                                .map(|s| { buffer.truncate(s); (i, buffer) })
                                .map_err(Into::into);
                            tx.send(result).await.unwrap()
                        }}});
                Ok(tap)
            }).collect::<Result<Vec<_>, SwitchrError>>()?;
            Ok::<_, SwitchrError>(taps)
        })?;

        Ok(TapHardware {
            rt: runtime,
            rx,
            taps
        })
    }
}

impl Hardware for TapHardware {
    fn send(&mut self, port_number: PortNumber, data: &[u8]) -> Result<(), SwitchrError> {
        self.rt.block_on(self.taps[port_number].send(data))?;
        Ok(())
    }

    fn recv(&mut self) -> Result<(PortNumber, Vec<u8>), SwitchrError> {
        self.rt.block_on(self.rx.recv()).unwrap_or(Err(SwitchrError::Closed))
    }
}

impl From<tokio_tun::Error> for SwitchrError {
    fn from(value: tokio_tun::Error) -> Self {
        SwitchrError::Generic(Box::new(value))
    }
}

impl From<std::io::Error> for SwitchrError {
    fn from(value: std::io::Error) -> Self {
        SwitchrError::Generic(Box::new(value))
    }
}