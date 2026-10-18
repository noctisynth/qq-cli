//! Local endpoint leases; no DNS, protocol bytes, retries or alternate fixed ports.
use crate::{
    error::{AppError, Result},
    runtime_config::Ports,
};
use std::net::{Ipv4Addr, TcpListener};
use std::time::Instant;

pub struct Lease {
    ports: Ports,
    _sockets: Vec<TcpListener>,
}
impl Lease {
    pub fn dynamic(deadline: Instant) -> Result<Self> {
        Self::bind([0, 0, 0], deadline)
    }
    pub fn acquire(ports: Ports, deadline: Instant) -> Result<Self> {
        ports.validate()?;
        Self::bind([ports.http, ports.events, ports.control], deadline)
    }
    pub fn ports(&self) -> Ports {
        self.ports
    }
    fn bind(ports: [u16; 3], deadline: Instant) -> Result<Self> {
        let mut sockets = Vec::new();
        for port in ports {
            crate::deadline::check(deadline)?;
            sockets.push(TcpListener::bind((Ipv4Addr::LOCALHOST,port)).map_err(|_| AppError::new(
                "ENDPOINT_IN_USE", "A planned local endpoint cannot be reserved; no alternate port or native process was selected.", 4))?);
        }
        let port = |index: usize| {
            sockets[index]
                .local_addr()
                .map(|a| a.port())
                .map_err(|_| AppError::storage())
        };
        let ports = Ports {
            http: port(0)?,
            events: port(1)?,
            control: port(2)?,
        };
        ports.validate()?;
        crate::deadline::check(deadline)?;
        Ok(Self {
            ports,
            _sockets: sockets,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn deadline() -> Instant {
        Instant::now() + std::time::Duration::from_secs(1)
    }
    #[test]
    fn fixed_lease_keeps_all_ports_and_releases_partial_acquisition_on_failure() {
        for occupied in [1, 2] {
            let lease = Lease::dynamic(deadline()).unwrap();
            let ports = lease.ports();
            drop(lease);
            let selected = [ports.http, ports.events, ports.control];
            let busy = TcpListener::bind((Ipv4Addr::LOCALHOST, selected[occupied])).unwrap();
            assert_eq!(
                Lease::acquire(ports, deadline()).err().unwrap().code,
                "ENDPOINT_IN_USE"
            );
            let prefix = TcpListener::bind((Ipv4Addr::LOCALHOST, selected[0])).unwrap();
            drop(prefix);
            drop(busy);
            let fixed = Lease::acquire(ports, deadline()).unwrap();
            assert_eq!(fixed.ports(), ports);
            for port in selected {
                assert!(TcpListener::bind((Ipv4Addr::LOCALHOST, port)).is_err());
            }
        }
    }
    #[test]
    fn expiry_and_invalid_fixed_plan_fail_without_alternate_ports() {
        assert_eq!(
            Lease::dynamic(Instant::now()).err().unwrap().code,
            "DEADLINE"
        );
        assert_eq!(
            Lease::acquire(
                Ports {
                    http: 19001,
                    events: 19001,
                    control: 19003
                },
                deadline()
            )
            .err()
            .unwrap()
            .code,
            "RUNTIME_CONFIG_INVALID"
        );
    }
}
