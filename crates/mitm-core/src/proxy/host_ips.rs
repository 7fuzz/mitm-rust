use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostIp {
    /// OS interface name, e.g. eth0, wlan0, lo
    pub interface: String,
    pub ip: String,
    pub is_ipv6: bool,
    pub is_loopback: bool,
    pub is_link_local: bool,
    pub is_up: bool,
}

/// Every interface address, most useful first: up, IPv4, routable, then loopback last.
pub fn list() -> Result<Vec<HostIp>, String> {
    let mut ips: Vec<HostIp> = if_addrs::get_if_addrs()
        .map_err(|e| format!("Failed to read network interfaces: {}", e))?
        .into_iter()
        .map(|iface| HostIp {
            is_ipv6: iface.ip().is_ipv6(),
            is_loopback: iface.is_loopback(),
            is_link_local: iface.is_link_local(),
            is_up: iface.is_oper_up(),
            ip: iface.ip().to_string(),
            interface: iface.name,
        })
        .collect();
    ips.sort_by_key(|h| (h.is_loopback, h.is_link_local, h.is_ipv6, !h.is_up));
    Ok(ips)
}

#[cfg(test)]
mod tests {
    #[test]
    fn lists_loopback_last() {
        let ips = super::list().unwrap();
        assert!(ips.iter().any(|h| h.is_loopback));
        let first_loopback = ips.iter().position(|h| h.is_loopback).unwrap();
        assert!(ips[first_loopback..].iter().all(|h| h.is_loopback));
    }
}
