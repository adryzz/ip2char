use bytes::Bytes;
use packet::ip::v4::Packet;
use packet::ip::{self};
use tokio::sync::broadcast;
use tracing::trace;
use tun::TunPacket;

pub fn handle_packet_from_kernel(
    data: Bytes,
    tx: &broadcast::Sender<Packet<Bytes>>,
) -> anyhow::Result<()> {
    match ip::Packet::new(data) {
        Ok(ip::Packet::V4(pkt)) => {
            tx.send(pkt)?;
        }
        Ok(ip::Packet::V6(_pkt)) => {
            //tracing::trace!("V6 packet, cant do anything about it for now");
        }
        Err(err) => println!("Received an invalid packet: {:?}", err),
    }

    Ok(())
}

pub fn prep_packet_for_kernel(packet: Bytes) -> anyhow::Result<TunPacket> {
    trace!("Sending packet to kernel");

    // ugh very bad for performance
    Ok(TunPacket::new(packet.to_vec()))
}
