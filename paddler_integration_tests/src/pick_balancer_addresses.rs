use anyhow::Result;

use crate::balancer_addresses::BalancerAddresses;
use crate::pick_free_port::pick_free_port;

pub fn pick_balancer_addresses() -> Result<BalancerAddresses> {
    Ok(BalancerAddresses {
        compat_openai: format!("127.0.0.1:{}", pick_free_port()?),
        inference: format!("127.0.0.1:{}", pick_free_port()?),
        management: format!("127.0.0.1:{}", pick_free_port()?),
    })
}
