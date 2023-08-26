use tokio_util::codec::Framed;
use tracing::info;
use tun::{AsyncDevice, TunPacketCodec};

use crate::config::Config;
use crate::MTU;

pub fn create_tun(config: &Config) -> anyhow::Result<Framed<AsyncDevice, TunPacketCodec>> {
    let mut tun_config = tun::Configuration::default();
    tun_config
        .address(config.interface.address.ip())
        .netmask(config.interface.address.mask())
        .name(&config.interface.name)
        .layer(tun::Layer::L3)
        .mtu(MTU as i32)
        .up();

    #[cfg(target_os = "linux")]
    tun_config.platform(|config| {
        config.packet_information(true);
    });

    let dev = tun::create_as_async(&tun_config)?;
    let framed = dev.into_framed();
    info!("[1] Created tun interface.");
    Ok(framed)
}
