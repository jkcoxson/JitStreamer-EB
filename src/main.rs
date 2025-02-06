// Jackson Coxson
// JitStreamer for the year of our Lord, 2025

const VERSION: [u8; 3] = [0, 1, 1];

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

use axum::{
    extract::{Json, Path, State},
    http::{header::CONTENT_TYPE, Method},
    routing::{get, post},
};
use axum_client_ip::SecureClientIp;
use heartbeat::NewHeartbeatSender;
use idevice::{
    installation_proxy::InstallationProxyClient, mounter::ImageMounter, pairing_file::PairingFile,
    provider::TcpProvider, IdeviceService,
};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

mod db;
mod debug_server;
mod heartbeat;
mod mount;
mod register;
mod runner;

#[tokio::main]
async fn main() {
    println!("Starting JitStreamer-EB, enabling logger");
    dotenvy::dotenv().ok();

    env_logger::init();
    info!("Logger initialized");

    // Run the environment checks
    register::check_wireguard();
    if !std::fs::exists("jitstreamer.db").unwrap() {
        info!("Creating database");
        let db = sqlite::open("jitstreamer.db").unwrap();
        db.execute(include_str!("sql/up.sql")).unwrap();
    }

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

    // Empty the queues
    mount::empty().await;
    debug_server::empty().await;

    // Create a heartbeat manager
    let heartbeat_sender = heartbeat::heartbeat();

    // Run the Python shims
    runner::run("src/runners/mount.py", runner_count);
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
        .route("/get_apps", get(get_apps))
        .route("/launch_app/{bundle_id}", get(launch_app))
        .route("/status", get(status))
        .with_state(heartbeat_sender);

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
    State(state): State<NewHeartbeatSender>,
) -> Json<GetAppsReturn> {
    let ip = ip.0;

    info!("Got request to get apps from {:?}", ip);

    let udid = match tokio::task::spawn_blocking(move || {
        let db = match sqlite::open("jitstreamer.db") {
            Ok(db) => db,
            Err(e) => {
                info!("Failed to open database: {:?}", e);
                return Err(Json(GetAppsReturn {
                    ok: false,
                    apps: Vec::new(),
                    bundle_ids: None,
                    error: Some(format!("Failed to open database: {:?}", e)),
                }));
            }
        };

        // Get the device from the database
        let query = "SELECT udid FROM devices WHERE ip = ?";
        let mut statement = match crate::db::db_prepare(&db, query) {
            Some(s) => s,
            None => {
                log::error!("Failed to prepare query!");
                return Err(Json(GetAppsReturn {
                    ok: false,
                    apps: Vec::new(),
                    bundle_ids: None,
                    error: Some("Failed to open database".to_string()),
                }));
            }
        };
        statement.bind((1, ip.to_string().as_str())).unwrap();
        let udid = if let Some(sqlite::State::Row) = crate::db::statement_next(&mut statement) {
            let udid = statement.read::<String, _>("udid").unwrap();
            info!("Found device with udid {}", udid);
            udid
        } else {
            info!("No device found for IP {:?}", ip);
            return Err(Json(GetAppsReturn {
                ok: false,
                apps: Vec::new(),
                bundle_ids: None,
                error: Some(format!("No device found for IP {:?}", ip)),
            }));
        };
        Ok(udid)
    })
    .await
    .unwrap()
    {
        Ok(udid) => udid,
        Err(e) => {
            return e;
        }
    };

    // Get the pairing file
    debug!("Getting pairing file for {udid}");
    let pairing_file = match get_pairing_file(udid.clone()).await {
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
    let apps: HashMap<String, String> = apps
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

    state
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
    mounting: bool,
    position: Option<usize>,
    error: Option<String>,
}
///  - Get the IP from the request and UDID from the database
/// - Make sure netmuxd still has the device
///  - Check the mounted images for the developer disk image
///    - If not mounted, add the device to the queue for mounting
///    - Return a message letting the user know the device is mounting
///  - Connect to tunneld and get the interface and port for the developer service
///  - Send the commands to launch the app and detach
///  - Set last_used to now in the database
async fn launch_app(
    ip: SecureClientIp,
    Path(bundle_id): Path<String>,
    State(state): State<heartbeat::NewHeartbeatSender>,
) -> Json<LaunchAppReturn> {
    let ip = ip.0;

    info!("Got request to launch {bundle_id} from {:?}", ip);

    let udid = match tokio::task::spawn_blocking(move || {
        let db = match sqlite::open("jitstreamer.db") {
            Ok(db) => db,
            Err(e) => {
                info!("Failed to open database: {:?}", e);
                return Err(Json(LaunchAppReturn {
                    ok: false,
                    launching: false,
                    mounting: false,
                    position: None,
                    error: Some(format!("Failed to open database: {:?}", e)),
                }));
            }
        };

        // Get the device from the database
        let query = "SELECT udid FROM devices WHERE ip = ?";
        let mut statement = match crate::db::db_prepare(&db, query) {
            Some(s) => s,
            None => {
                log::error!("Failed to prepare query!");
                return Err(Json(LaunchAppReturn {
                    ok: false,
                    launching: false,
                    mounting: false,
                    position: None,
                    error: Some("Failed to open database".to_string()),
                }));
            }
        };
        statement.bind((1, ip.to_string().as_str())).unwrap();
        let udid = if let Some(sqlite::State::Row) = crate::db::statement_next(&mut statement) {
            let udid = statement.read::<String, _>("udid").unwrap();
            info!("Found device with udid {}", udid);
            udid
        } else {
            info!("No device found for IP {:?}", ip);
            return Err(Json(LaunchAppReturn {
                ok: false,
                launching: false,
                mounting: false,
                position: None,
                error: Some("No device found in database".to_string()),
            }));
        };
        Ok(udid)
    })
    .await
    .unwrap()
    {
        Ok(udid) => udid,
        Err(e) => {
            return e;
        }
    };

    // Check if there are any launches queued
    debug!("Checking launch queue for {udid}");
    match debug_server::get_queue_info(&udid).await {
        debug_server::LaunchQueueInfo::Position(p) => {
            return Json(LaunchAppReturn {
                ok: true,
                launching: true,
                mounting: false,
                position: Some(p),
                error: None,
            });
        }
        debug_server::LaunchQueueInfo::NotInQueue => {}
        debug_server::LaunchQueueInfo::Error(e) => {
            return Json(LaunchAppReturn {
                ok: false,
                launching: false,
                mounting: false,
                position: None,
                error: Some(e),
            });
        }
        debug_server::LaunchQueueInfo::ServerError => {
            return Json(LaunchAppReturn {
                ok: false,
                launching: false,
                mounting: false,
                position: None,
                error: Some("Failed to get launch status".to_string()),
            });
        }
    }

    // Check the mounting status
    match mount::get_queue_info(&udid).await {
        mount::MountQueueInfo::Position(p) => {
            return Json(LaunchAppReturn {
                ok: true,
                launching: false,
                mounting: true,
                position: Some(p),
                error: None,
            });
        }
        mount::MountQueueInfo::NotInQueue => {}
        mount::MountQueueInfo::Error(e) => {
            return Json(LaunchAppReturn {
                ok: false,
                launching: false,
                mounting: false,
                position: None,
                error: Some(e),
            });
        }
        mount::MountQueueInfo::ServerError => {
            return Json(LaunchAppReturn {
                ok: false,
                launching: false,
                mounting: false,
                position: None,
                error: Some("Failed to get mounting status".to_string()),
            });
        }
        mount::MountQueueInfo::InProgress => {
            return Json(LaunchAppReturn {
                ok: true,
                launching: false,
                mounting: true,
                position: None,
                error: None,
            });
        }
    }

    // Get the pairing file
    let pairing_file = match get_pairing_file(udid.clone()).await {
        Ok(pairing_file) => pairing_file,
        Err(e) => {
            info!("Failed to get pairing file: {:?}", e);
            return Json(LaunchAppReturn {
                ok: false,
                launching: false,
                mounting: false,
                position: None,
                error: Some(format!("Failed to get pairing file: {:?}", e)),
            });
        }
    };

    // Heartbeat the device
    match heartbeat::heartbeat_thread(udid.clone(), ip, &pairing_file).await {
        Ok(s) => {
            state
                .send(heartbeat::SendRequest::Store((udid.clone(), s)))
                .await
                .unwrap();
        }
        Err(e) => {
            info!("Failed to heartbeat device: {:?}", e);
            return Json(LaunchAppReturn {
                ok: false,
                launching: false,
                mounting: false,
                position: None,
                error: Some(format!("Failed to heartbeat device: {:?}", e)),
            });
        }
    }

    // Get the list of mounted images
    let provider = TcpProvider {
        addr: ip,
        pairing_file,
        label: "JitStreamer-EB".to_string(),
    };

    let mut mounter_client = match ImageMounter::connect(&provider).await {
        Ok(m) => m,
        Err(e) => {
            return Json(LaunchAppReturn {
                ok: false,
                launching: false,
                mounting: false,
                position: None,
                error: Some(format!("Failed to start image mounter: {e:?}")),
            })
        }
    };

    let images = match mounter_client.copy_devices().await {
        Ok(images) => images,
        Err(e) => {
            info!("Failed to get images: {:?}", e);
            return Json(LaunchAppReturn {
                ok: false,
                launching: false,
                mounting: false,
                position: None,
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

    if !mounted {
        // Add the device to the queue for mounting
        mount::add_to_queue(&udid, ip.to_string()).await;

        // Return a message letting the user know the device is mounting
        return Json(LaunchAppReturn {
            ok: true,
            launching: false,
            mounting: true,
                position: None,
            error: Some("Your device has been added to the queue for image mounting. Please try again in a minute.".to_string()),
        });
    }

    state
        .send(heartbeat::SendRequest::Kill(udid.clone()))
        .await
        .unwrap();

    // Add the launch to the queue
    match debug_server::add_to_queue(&udid, ip.to_string(), &bundle_id).await {
        Some(position) => Json(LaunchAppReturn {
            ok: true,
            launching: true,
            mounting: false,
            position: Some(position as usize),
            error: None,
        }),
        None => Json(LaunchAppReturn {
            ok: false,
            launching: false,
            mounting: false,
            position: None,
            error: Some("Failed to add to queue".to_string()),
        }),
    }
}

#[derive(Debug, Serialize)]
struct StatusReturn {
    done: bool,
    ok: bool,
    position: usize,
    in_progress: bool,
    error: Option<String>,
}

/// Gets the current status of the device
/// Returns immediately if done or error
/// Checks every second, up to 15 seconds for a new response.
async fn status(ip: SecureClientIp) -> Json<StatusReturn> {
    let start_time = std::time::Instant::now();
    let ip = ip.0;

    let udid = match tokio::task::spawn_blocking(move || {
        let db = match sqlite::open("jitstreamer.db") {
            Ok(db) => db,
            Err(e) => {
                info!("Failed to open database: {:?}", e);
                return Err(Json(StatusReturn {
                    done: true,
                    ok: false,
                    position: 0,
                    in_progress: false,
                    error: Some(format!("Failed to open database: {:?}", e)),
                }));
            }
        };

        // Get the device from the database
        let query = "SELECT udid FROM devices WHERE ip = ?";
        let mut statement = match crate::db::db_prepare(&db, query) {
            Some(s) => s,
            None => {
                log::error!("Failed to prepare query!");
                return Err(Json(StatusReturn {
                    ok: false,
                    done: false,
                    in_progress: false,
                    position: 0,
                    error: Some("Failed to open database".to_string()),
                }));
            }
        };
        statement.bind((1, ip.to_string().as_str())).unwrap();
        let udid = if let Some(sqlite::State::Row) = crate::db::statement_next(&mut statement) {
            let udid = statement.read::<String, _>("udid").unwrap();
            info!("Found device with udid {}", udid);
            udid
        } else {
            info!("No device found for IP {:?}", ip);
            return Err(Json(StatusReturn {
                done: true,
                ok: false,
                position: 0,
                in_progress: false,
                error: Some("No device found in database".to_string()),
            }));
        };
        Ok(udid)
    })
    .await
    .unwrap()
    {
        Ok(udid) => udid,
        Err(e) => {
            return e;
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
                    in_progress: false,
                    error: None,
                }));
            }
            debug_server::LaunchQueueInfo::NotInQueue => {}
            debug_server::LaunchQueueInfo::Error(e) => {
                to_return = Some(Json(StatusReturn {
                    ok: false,
                    done: true,
                    position: 0,
                    in_progress: false,
                    error: Some(e),
                }));
            }
            debug_server::LaunchQueueInfo::ServerError => {
                to_return = Some(Json(StatusReturn {
                    ok: false,
                    done: true,
                    position: 0,
                    in_progress: false,
                    error: Some("server error".to_string()),
                }));
            }
        }

        // Check the mounting status
        match mount::get_queue_info(&udid).await {
            mount::MountQueueInfo::Position(p) => {
                to_return = Some(Json(StatusReturn {
                    ok: true,
                    done: false,
                    position: p,
                    in_progress: false,
                    error: None,
                }));
            }
            mount::MountQueueInfo::NotInQueue => {}
            mount::MountQueueInfo::Error(e) => {
                to_return = Some(Json(StatusReturn {
                    ok: false,
                    done: true,
                    position: 0,
                    in_progress: false,
                    error: Some(e),
                }));
            }
            mount::MountQueueInfo::ServerError => {
                to_return = Some(Json(StatusReturn {
                    ok: false,
                    done: true,
                    position: 0,
                    in_progress: false,
                    error: Some("server error".to_string()),
                }));
            }
            mount::MountQueueInfo::InProgress => {
                to_return = Some(Json(StatusReturn {
                    ok: true,
                    done: false,
                    position: 0,
                    in_progress: true,
                    error: None,
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
                        in_progress: false,
                        error: None,
                    });
                }
            }
        }
    }
}

/// Gets the pairing file
async fn get_pairing_file(udid: String) -> Result<PairingFile, idevice::IdeviceError> {
    // All pairing files are stored at /var/lib/lockdown/<udid>.plist
    let path = format!("/var/lib/lockdown/{}.plist", udid);
    let pairing_file = tokio::fs::read(path).await?;

    PairingFile::from_bytes(&pairing_file)
}
