//! Mock TCP **backend** that drives recorded `backend_to_client` / expects `client_to_backend` events.

use crate::protocol::codec::PacketOrAck;
use crate::protocol::Packet;
use crate::recorder::{RecordDirection, RecordEventV1, RecordKind};
use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

use crate::protocol::codec::GdbCodec;

use super::ReplayError;

fn item_summary(item: &PacketOrAck) -> String {
    match item {
        PacketOrAck::Ack => "+".into(),
        PacketOrAck::Nack => "-".into(),
        PacketOrAck::Packet(p) => format!("Packet({})", String::from_utf8_lossy(&p.data)),
    }
}

/// Convert a recorded event to the wire [`PacketOrAck`] (same shape as the proxy codec).
pub fn event_to_item(ev: &RecordEventV1) -> Result<PacketOrAck, ReplayError> {
    match ev.kind {
        RecordKind::Ack => Ok(PacketOrAck::Ack),
        RecordKind::Nack => Ok(PacketOrAck::Nack),
        RecordKind::Packet => {
            let hex = ev
                .payload_hex
                .as_ref()
                .ok_or_else(|| ReplayError::InvalidEvent("packet missing payload_hex".into()))?;
            let data = hex::decode(hex.trim())?;
            Ok(PacketOrAck::Packet(Packet::new(data)))
        }
    }
}

/// Run one mock-backend session: the **proxy** must connect as the TCP client (same as a real stub).
///
/// Processes events in file order: for each `client_to_backend` entry, reads one framed item from the
/// proxy and checks equality; for each `backend_to_client` entry, writes one framed item to the proxy.
pub async fn run_mock_backend(
    socket: TcpStream,
    events: Vec<RecordEventV1>,
) -> Result<(), ReplayError> {
    let mut framed = Framed::new(socket, GdbCodec::new());

    for (i, ev) in events.iter().enumerate() {
        let step = i + 1;
        match ev.direction {
            RecordDirection::BackendToClient => {
                let item = event_to_item(ev)?;
                framed.send(item).await?;
            }
            RecordDirection::ClientToBackend => {
                let expected = event_to_item(ev)?;
                let got = framed
                    .next()
                    .await
                    .transpose()?
                    .ok_or(ReplayError::UnexpectedEof { step })?;
                if got != expected {
                    return Err(ReplayError::Mismatch {
                        step,
                        expected: item_summary(&expected),
                        got: item_summary(&got),
                    });
                }
            }
        }
    }

    Ok(())
}
