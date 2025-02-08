// Jackson Coxson
// Orchestrator for heartbeat threads

use std::{collections::HashMap, net::IpAddr};

use idevice::{
    heartbeat::HeartbeatClient, pairing_file::PairingFile, provider::TcpProvider, IdeviceError,
    IdeviceService,
};
use log::debug;
use tokio::sync::oneshot::error::TryRecvError;

pub enum SendRequest {
    Store((String, tokio::sync::oneshot::Sender<()>)),
    Kill(String),
}
pub type NewHeartbeatSender = tokio::sync::mpsc::Sender<SendRequest>;

pub fn heartbeat() -> NewHeartbeatSender {
    let (sender, mut receiver) = tokio::sync::mpsc::channel::<SendRequest>(100);
    tokio::task::spawn(async move {
        let mut cache: HashMap<String, tokio::sync::oneshot::Sender<()>> = HashMap::new();
        while let Some(msg) = receiver.recv().await {
            match msg {
                SendRequest::Store((udid, handle)) => {
                    if let Some(old_sender) = cache.insert(udid, handle) {
                        old_sender.send(()).ok();
                    }
                }
                SendRequest::Kill(udid) => {
                    if let Some(old_sender) = cache.remove(&udid) {
                        old_sender.send(()).ok();
                    }
                }
            }
        }
    });
    sender
}

pub async fn heartbeat_thread(
    udid: String,
    ip: IpAddr,
    pairing_file: &PairingFile,
) -> Result<tokio::sync::oneshot::Sender<()>, IdeviceError> {
    debug!("Connecting to device {udid} to get apps");
    let provider = TcpProvider {
        addr: ip,
        pairing_file: pairing_file.clone(),
        label: "JitStreamer-EB".to_string(),
    };

    let mut heartbeat_client = HeartbeatClient::connect(&provider).await?;

    let (sender, mut receiver) = tokio::sync::oneshot::channel::<()>();

    tokio::task::spawn(async move {
        let interval = 30;
        loop {
            let _ = match heartbeat_client.get_marco(interval).await {
                Ok(interval) => interval,
                Err(e) => {
                    debug!("Failed to get marco for {udid}: {e:?}");
                    break;
                }
            };
            if heartbeat_client.send_polo().await.is_err() {
                debug!("Failed to send polo for {udid}");
                break;
            }
            match receiver.try_recv() {
                Ok(_) => break,
                Err(TryRecvError::Closed) => break,
                Err(TryRecvError::Empty) => {}
            }
        }
    });
    Ok(sender)
}
