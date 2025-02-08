// Jackson Coxson

use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    Json,
};
use axum_client_ip::SecureClientIp;
use idevice::{
    lockdownd::LockdowndClient,
    mounter::ImageMounter,
    provider::{IdeviceProvider, TcpProvider},
    IdeviceError, IdeviceService,
};
use log::{debug, info, warn};
use serde::Serialize;
use tokio::sync::{watch, Mutex};

use crate::{
    common,
    heartbeat::{self, NewHeartbeatSender},
    JitStreamerState,
};

const BUILD_MANIFEST: &[u8] = include_bytes!("../DDI/BuildManifest.plist");
const DDI_IMAGE: &[u8] = include_bytes!("../DDI/Image.dmg");
const DDI_TRUSTCACHE: &[u8] = include_bytes!("../DDI/Image.dmg.trustcache");

pub type MountCache =
    Arc<Mutex<HashMap<String, watch::Receiver<Result<(usize, usize, bool), String>>>>>;

#[derive(Serialize)]
pub struct CheckMountResponse {
    ok: bool,
    error: Option<String>,
    mounting: bool,
}

#[derive(Serialize)]
pub struct MountWebSocketMessage {
    ok: bool,
    percentage: f32,
    error: Option<String>,
    done: bool,
}

pub async fn check_mount(
    ip: SecureClientIp,
    State(state): State<JitStreamerState>,
) -> Json<CheckMountResponse> {
    let udid = match common::get_udid_from_ip(ip.0.to_string()).await {
        Ok(u) => u,
        Err(e) => {
            return Json(CheckMountResponse {
                ok: false,
                error: Some(e),
                mounting: false,
            });
        }
    };

    let mut lock = state.mount_cache.lock().await;
    if let Some(i) = lock.get(&udid) {
        let i = i.borrow().clone();
        match i {
            Ok((_, _, complete)) => {
                if complete {
                    lock.remove(&udid);
                    return Json(CheckMountResponse {
                        ok: true,
                        error: None,
                        mounting: false,
                    });
                }
            }
            Err(e) => {
                lock.remove(&udid);
                return Json(CheckMountResponse {
                    ok: false,
                    error: Some(format!("Failed to mount image: {e}")),
                    mounting: false,
                });
            }
        }
        debug!("Device {udid} is already mounting");
        return Json(CheckMountResponse {
            ok: true,
            error: None,
            mounting: true,
        });
    }
    std::mem::drop(lock);

    let pairing_file = match common::get_pairing_file(&udid).await {
        Ok(p) => p,
        Err(e) => {
            return Json(CheckMountResponse {
                ok: false,
                mounting: false,
                error: Some(format!("Unable to get pairing file: {e}")),
            })
        }
    };

    // Start a heartbeat, get the list of images
    match heartbeat::heartbeat_thread(udid.clone(), ip.0, &pairing_file).await {
        Ok(s) => {
            state
                .new_heartbeat_sender
                .send(heartbeat::SendRequest::Store((udid.clone(), s)))
                .await
                .unwrap();
        }
        Err(e) => {
            let e = match e {
                idevice::IdeviceError::InvalidHostID => {
                    "your pairing file is invalid. Regenerate it with jitterbug pair.".to_string()
                }
                _ => e.to_string(),
            };
            info!("Failed to heartbeat device: {:?}", e);
            return Json(CheckMountResponse {
                ok: false,
                mounting: false,
                error: Some(format!("Failed to heartbeat device: {e}")),
            });
        }
    }

    // Get the list of mounted images
    let provider = TcpProvider {
        addr: ip.0,
        pairing_file,
        label: "JitStreamer-EB".to_string(),
    };

    let mut mounter_client = match ImageMounter::connect(&provider).await {
        Ok(m) => m,
        Err(e) => {
            return Json(CheckMountResponse {
                ok: false,
                mounting: false,
                error: Some(format!("Failed to start image mounter: {e:?}")),
            })
        }
    };

    let images = match mounter_client.copy_devices().await {
        Ok(images) => images,
        Err(e) => {
            info!("Failed to get images: {:?}", e);
            return Json(CheckMountResponse {
                ok: false,
                mounting: false,
                error: Some(format!("Failed to get images: {:?}", e)),
            });
        }
    };

    let mut mounted = false;
    for image in images {
        let mut buf = Vec::new();
        let mut writer = std::io::Cursor::new(&mut buf);
        plist::to_writer_xml(&mut writer, &image).unwrap();

        let image = String::from_utf8_lossy(&buf);
        if image.contains("Developer") {
            mounted = true;
            break;
        }
    }

    if mounted {
        Json(CheckMountResponse {
            ok: true,
            error: None,
            mounting: false,
        })
    } else {
        let (sw, rw) = watch::channel(Ok((0, 100, false)));
        mount_thread(
            provider,
            sw,
            state.new_heartbeat_sender.clone(),
            udid.clone(),
        );
        state.mount_cache.lock().await.insert(udid, rw);

        Json(CheckMountResponse {
            ok: true,
            error: None,
            mounting: true,
        })
    }
}

fn mount_thread(
    provider: TcpProvider,
    sender: watch::Sender<Result<(usize, usize, bool), String>>,
    hb: NewHeartbeatSender,
    udid: String,
) {
    debug!("Starting mount thread for {udid}");
    tokio::task::spawn(async move {
        // Start work in a new fuction so we can use ?
        async fn work(
            provider: TcpProvider,
            sender: watch::Sender<Result<(usize, usize, bool), String>>,
            hb: NewHeartbeatSender,
            udid: String,
        ) -> Result<(), IdeviceError> {
            debug!("Getting chip ID for {udid}");
            let mut lockdown_client = LockdowndClient::connect(&provider).await?;
            lockdown_client
                .start_session(&provider.get_pairing_file().await?)
                .await?;

            let unique_chip_id = match lockdown_client
                .get_value("UniqueChipID")
                .await?
                .as_unsigned_integer()
            {
                Some(u) => u,
                None => {
                    return Err(IdeviceError::UnexpectedResponse);
                }
            };

            let mut mounter_client = ImageMounter::connect(&provider).await?;
            mounter_client
                .mount_personalized_with_callback(
                    &provider,
                    DDI_IMAGE.to_vec(),
                    DDI_TRUSTCACHE.to_vec(),
                    BUILD_MANIFEST,
                    None,
                    unique_chip_id,
                    |(progress, state)| async move {
                        state.clone().send(Ok((progress.0, progress.1, false))).ok();
                    },
                    sender,
                )
                .await?;
            hb.send(crate::heartbeat::SendRequest::Kill(udid))
                .await
                .ok();
            Ok(())
        }
        if let Err(e) = work(provider, sender.clone(), hb, udid.clone()).await {
            warn!("Failed to mount for {udid}: {e:?}");
            sender.send(Err(e.to_string())).ok();
        } else {
            sender.send(Ok((1, 1, true))).ok();
        }
    });
}

pub async fn handler(
    ws: WebSocketUpgrade,
    ip: SecureClientIp,
    State(state): State<JitStreamerState>,
) -> axum::response::Response {
    let ip = ip.0.to_string();
    ws.on_upgrade(|s| async move { handle_socket(s, ip.clone(), state).await })
}

async fn handle_socket(mut socket: WebSocket, ip: String, state: JitStreamerState) {
    let udid = match common::get_udid_from_ip(ip).await {
        Ok(u) => u,
        Err(e) => {
            socket
                .send(
                    MountWebSocketMessage {
                        ok: false,
                        percentage: 0.0,
                        error: Some(e),
                        done: false,
                    }
                    .to_ws_message(),
                )
                .await
                .ok();
            return;
        }
    };

    let lock = state.mount_cache.lock().await;
    let mut receiver = match lock.get(&udid) {
        Some(r) => r.clone(),
        None => {
            socket
                .send(
                    MountWebSocketMessage {
                        ok: true,
                        error: None,
                        percentage: 0.0,
                        done: false,
                    }
                    .to_ws_message(),
                )
                .await
                .ok();
            return;
        }
    };
    std::mem::drop(lock);

    loop {
        let msg = receiver.borrow().clone();
        if match msg {
            Ok((a, b, complete)) => socket.send(
                MountWebSocketMessage {
                    ok: true,
                    error: None,
                    percentage: a as f32 / b as f32,
                    done: complete,
                }
                .to_ws_message(),
            ),
            Err(e) => socket.send(
                MountWebSocketMessage {
                    ok: false,
                    error: Some(e),
                    percentage: 0.0,
                    done: false,
                }
                .to_ws_message(),
            ),
        }
        .await
        .is_err()
        {
            debug!("Failed to send status to websocket");
            return;
        }

        if receiver.changed().await.is_err() {
            debug!("Receiver failed to recv msg");
            return;
        }
    }
}

impl MountWebSocketMessage {
    fn to_ws_message(&self) -> Message {
        Message::text(serde_json::to_string(&self).unwrap())
    }
}
