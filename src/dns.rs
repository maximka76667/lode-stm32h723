use embassy_net::{IpAddress, Stack, dns::DnsQueryType};

/// Resolve a hostname to its first A-record using the DHCP-provided DNS server.
/// Returns `None` if the query fails or returns no addresses.
pub async fn resolve(stack: Stack<'static>, host: &str) -> Option<IpAddress> {
    match stack.dns_query(host, DnsQueryType::A).await {
        Ok(addrs) => addrs.first().copied(),
        Err(_) => None,
    }
}

/// Resolve `host`, retrying every `retry_secs` seconds until successful.
pub async fn resolve_with_retry(stack: Stack<'static>, host: &str, retry_secs: u64) -> IpAddress {
    loop {
        match resolve(stack, host).await {
            Some(addr) => return addr,
            None => embassy_time::Timer::after_secs(retry_secs).await,
        }
    }
}
