// Jackson Coxson
// JitStreamer for the year of our Lord, 2025

const VERSION: [u8; 3] = [0, 2, 0];

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

use axum::{
    extract::{Json, Path, State},
    http::{header::CONTENT_TYPE, Method},
    response::Html,
    routing::{any, get, post},
};
use axum_client_ip::SecureClientIp;
use common::get_pairing_file;
use heartbeat::NewHeartbeatSender;
use idevice::{installation_proxy::InstallationProxyClient, provider::TcpProvider, IdeviceService};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

mod common;
mod db;
mod debug_server;
mod heartbeat;
mod mount;
mod register;
mod runner;

#[derive(Clone)]
struct JitStreamerState {
    pub new_heartbeat_sender: NewHeartbeatSender,
    pub mount_cache: mount::MountCache,
}

#[tokio::main]
async fn main() {
    println!("Starting JitStreamer-EB, enabling logger");
    dotenvy::dotenv().ok();
    //
    // Read the environment variable constants
    let runner_count = std::env::var("RUNNER_COUNT")
        .unwrap_or("10".to_string())
        .parse::<u32>()
        .unwrap();
    let allow_registration = std::env::var("ALLOW_REGISTRATION")
        .unwrap_or("1".to_string())
        .parse::<u8>()
        .unwrap();
    let port = std::env::var("JITSTREAMER_PORT")
        .unwrap_or("9172".to_string())
        .parse::<u16>()
        .unwrap();

    env_logger::init();
    info!("Logger initialized");

    // Run the environment checks
    if allow_registration == 1 {
        register::check_wireguard();
    }
    if !std::fs::exists("jitstreamer.db").unwrap() {
        info!("Creating database");
        let db = sqlite::open("jitstreamer.db").unwrap();
        db.execute(include_str!("sql/up.sql")).unwrap();
    }

    // Empty the queues
    debug_server::empty().await;

    // Create a heartbeat manager
    let state = JitStreamerState {
        new_heartbeat_sender: heartbeat::heartbeat(),
        mount_cache: mount::MountCache::default(),
    };

    // Run the Python shims
    runner::run("src/runners/launch.py", runner_count);

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_origin(tower_http::cors::Any)
        .allow_headers([CONTENT_TYPE]);

    // Start with Axum
    let app = axum::Router::new()
        .layer(cors.clone())
        .route("/hello", get(|| async { "Hello, world!" }))
        .route("/version", post(version))
        .route("/mount", get(mount::check_mount))
        .route("/mount_ws", any(mount::handler))
        .route(
            "/mount_status",
            get(|| async { Html(include_str!("mount.html")) }),
        )
        .route("/get_apps", get(get_apps))
        .route("/launch_app/{bundle_id}", get(launch_app))
        .route("/status", get(status))
        .with_state(state);

    let app = if allow_registration == 1 {
        app.route("/register", post(register::register))
    } else if allow_registration == 2 {
        app.route("/register", post(register::register))
            .route("/upload", get(register::upload))
    } else {
        app
    };

    let app = app
        .layer(axum_client_ip::SecureClientIpSource::ConnectInfo.into_extension())
        .layer(cors);

    let addr = SocketAddr::new(IpAddr::from_str("::0").unwrap(), port);
    info!("Starting server on {:?}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

#[derive(Serialize, Deserialize)]
struct VersionRequest {
    version: String,
}
#[derive(Serialize, Deserialize)]
struct VersionResponse {
    ok: bool,
}

async fn version(Json(version): Json<VersionRequest>) -> Json<VersionResponse> {
    info!("Checking version {}", version.version);

    // Parse the version as 3 numbers
    let version = version
        .version
        .split('.')
        .map(|v| v.parse::<u8>().unwrap_or(0))
        .collect::<Vec<u8>>();

    // Compare the version, compare each number
    for (i, v) in VERSION.iter().enumerate() {
        if version.get(i).unwrap_or(&0) < v {
            return Json(VersionResponse { ok: false });
        }
    }

    Json(VersionResponse { ok: true })
}

#[derive(Serialize, Deserialize, Clone)]
struct GetAppsReturn {
    ok: bool,
    apps: Vec<String>,
    bundle_ids: Option<HashMap<String, String>>,
    error: Option<String>,
}

/// Gets the list of apps with get-task-allow on the device
///  - Get the IP from the request and UDID from the database
///  - Send the udid/IP to netmuxd for heartbeat-ing
///  - Connect to the device and get the list of bundle IDs
#[axum::debug_handler]
async fn get_apps(
    ip: SecureClientIp,
    State(state): State<JitStreamerState>,
) -> Json<GetAppsReturn> {
    let ip = ip.0;

    info!("Got request to get apps from {:?}", ip);

    let udid = match common::get_udid_from_ip(ip.to_string()).await {
        Ok(u) => u,
        Err(e) => {
            return Json(GetAppsReturn {
                ok: false,
                apps: Vec::new(),
                bundle_ids: None,
                error: Some(e),
            })
        }
    };

    // Get the pairing file
    debug!("Getting pairing file for {udid}");
    let pairing_file = match get_pairing_file(&udid).await {
        Ok(pairing_file) => pairing_file,
        Err(e) => {
            info!("Failed to get pairing file: {:?}", e);
            return Json(GetAppsReturn {
                ok: false,
                apps: Vec::new(),
                bundle_ids: None,
                error: Some(format!("Failed to get pairing file: {:?}", e)),
            });
        }
    };

    // Heartbeat the device
    match heartbeat::heartbeat_thread(udid.clone(), ip, &pairing_file).await {
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
            return Json(GetAppsReturn {
                ok: false,
                apps: Vec::new(),
                bundle_ids: None,
                error: Some(format!("Failed to heartbeat device: {e}")),
            });
        }
    }

    // Connect to the device and get the list of bundle IDs
    debug!("Connecting to device {udid} to get apps");

    let provider = TcpProvider {
        addr: ip,
        pairing_file,
        label: "JitStreamer-EB".to_string(),
    };

    let mut instproxy_client = match InstallationProxyClient::connect(&provider).await {
        Ok(i) => i,
        Err(e) => {
            return Json(GetAppsReturn {
                ok: false,
                apps: Vec::new(),
                bundle_ids: None,
                error: Some(format!("Failed to start instproxy: {e:?}")),
            })
        }
    };

    let apps = match instproxy_client
        .get_apps(Some("User".to_string()), None)
        .await
    {
        Ok(apps) => apps,
        Err(e) => {
            info!("Failed to get apps: {:?}", e);
            return Json(GetAppsReturn {
                ok: false,
                apps: Vec::new(),
                bundle_ids: None,
                error: Some(format!("Failed to get apps: {:?}", e)),
            });
        }
    };
    let mut apps: HashMap<String, String> = apps
        .into_iter()
        .filter(|(_, app)| {
            // Filter out apps that don't have get-task-allow
            let app = match app {
                plist::Value::Dictionary(app) => app,
                _ => return false,
            };

            match app.get("Entitlements") {
                Some(plist::Value::Dictionary(entitlements)) => {
                    matches!(
                        entitlements.get("get-task-allow"),
                        Some(plist::Value::Boolean(true))
                    )
                }
                _ => false,
            }
        })
        .map(|(bundle_id, app)| {
            let name = match app {
                plist::Value::Dictionary(mut d) => match d.remove("CFBundleName") {
                    Some(plist::Value::String(bundle_name)) => bundle_name,
                    _ => bundle_id.clone(),
                },
                _ => bundle_id.clone(),
            };
            (name.clone(), bundle_id)
        })
        .collect();

    if apps.is_empty() {
        return Json(GetAppsReturn {
            ok: false,
            apps: Vec::new(),
            bundle_ids: None,
            error: Some("No apps with get-task-allow found".to_string()),
        });
    }

    apps.insert("Other...".to_string(), "UPDATE YOUR SHORTCUT".to_string());

    state
        .new_heartbeat_sender
        .send(heartbeat::SendRequest::Kill(udid.clone()))
        .await
        .unwrap();

    Json(GetAppsReturn {
        ok: true,
        apps: apps.keys().map(|x| x.to_string()).collect(),
        bundle_ids: Some(apps),
        error: None,
    })
}

#[derive(Serialize, Deserialize)]
struct LaunchAppReturn {
    ok: bool,
    launching: bool,
    position: Option<usize>,
    error: Option<String>,
    mounting: bool, // NOTICE: this field does literally nothing and will be removed in future
                    // versions
}
///  - Get the IP from the request and UDID from the database
/// - Make sure netmuxd still has the device
///  - Check the mounted images for the developer disk image
///    - If not mounted, add the device to the queue for mounting
///    - Return a message letting the user know the device is mounting
///  - Connect to tunneld and get the interface and port for the developer service
///  - Send the commands to launch the app and detach
///  - Set last_used to now in the database
async fn launch_app(ip: SecureClientIp, Path(bundle_id): Path<String>) -> Json<LaunchAppReturn> {
    let ip = ip.0;

    info!("Got request to launch {bundle_id} from {:?}", ip);

    let udid = match common::get_udid_from_ip(ip.to_string()).await {
        Ok(u) => u,
        Err(e) => {
            return Json(LaunchAppReturn {
                ok: false,
                error: Some(e),
                launching: false,
                position: None,
                mounting: false,
            })
        }
    };

    // Check if there are any launches queued
    debug!("Checking launch queue for {udid}");
    match debug_server::get_queue_info(&udid).await {
        debug_server::LaunchQueueInfo::Position(p) => {
            return Json(LaunchAppReturn {
                ok: true,
                launching: true,
                position: Some(p),
                error: None,
                mounting: false,
            });
        }
        debug_server::LaunchQueueInfo::NotInQueue => {}
        debug_server::LaunchQueueInfo::Error(e) => {
            return Json(LaunchAppReturn {
                ok: false,
                launching: false,
                position: None,
                error: Some(e),
                mounting: false,
            });
        }
        debug_server::LaunchQueueInfo::ServerError => {
            return Json(LaunchAppReturn {
                ok: false,
                launching: false,
                position: None,
                error: Some("Failed to get launch status".to_string()),
                mounting: false,
            });
        }
    }

    // Add the launch to the queue
    match debug_server::add_to_queue(&udid, ip.to_string(), &bundle_id).await {
        Some(position) => Json(LaunchAppReturn {
            ok: true,
            launching: true,
            position: Some(position as usize),
            error: None,
            mounting: false,
        }),
        None => Json(LaunchAppReturn {
            ok: false,
            launching: false,
            position: None,
            error: Some("Failed to add to queue".to_string()),
            mounting: false,
        }),
    }
}

#[derive(Debug, Serialize)]
struct StatusReturn {
    done: bool,
    ok: bool,
    position: usize,
    error: Option<String>,
    in_progress: bool, // NOTICE: this field is deprecated and will be removed in future versions
}

/// Gets the current status of the device
/// Returns immediately if done or error
/// Checks every second, up to 15 seconds for a new response.
async fn status(ip: SecureClientIp) -> Json<StatusReturn> {
    let start_time = std::time::Instant::now();
    let ip = ip.0;

    let udid = match common::get_udid_from_ip(ip.to_string()).await {
        Ok(u) => u,
        Err(e) => {
            return Json(StatusReturn {
                ok: false,
                done: true,
                error: Some(e),
                position: 0,
                in_progress: false,
            })
        }
    };

    loop {
        // Check mounts
        // Check launches
        // Check if it's been too long
        let mut to_return = None;
        match debug_server::get_queue_info(&udid).await {
            debug_server::LaunchQueueInfo::Position(p) => {
                to_return = Some(Json(StatusReturn {
                    ok: true,
                    done: false,
                    position: p,
                    error: None,
                    in_progress: false,
                }));
            }
            debug_server::LaunchQueueInfo::NotInQueue => {}
            debug_server::LaunchQueueInfo::Error(e) => {
                to_return = Some(Json(StatusReturn {
                    ok: false,
                    done: true,
                    position: 0,
                    error: Some(e),
                    in_progress: false,
                }));
            }
            debug_server::LaunchQueueInfo::ServerError => {
                to_return = Some(Json(StatusReturn {
                    ok: false,
                    done: true,
                    position: 0,
                    error: Some("server error".to_string()),
                    in_progress: false,
                }));
            }
        }

        match to_return {
            Some(to_return) => {
                if start_time.elapsed() > std::time::Duration::from_secs(15) || to_return.done {
                    info!("Returning status for {udid}: {to_return:?}");
                    return to_return;
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            None => {
                if to_return.is_none() {
                    return Json(StatusReturn {
                        ok: true,
                        done: true,
                        position: 0,
                        error: None,
                        in_progress: false,
                    });
                }
            }
        }
    }
}
