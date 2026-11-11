#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod appearance;
mod bug_report;
mod capture;
mod ccm;
mod collector;
mod config;
mod desk;
mod desk_text;
mod drop_paths;
mod episodes;
mod files_ipc;
mod guest;
mod guest_ipc;
mod host_devices;
mod host_extra;
mod host_run;
mod hosts;
mod legacy;
mod lifecycle;
mod links;
mod log;
mod mcp;
mod native;
mod notice;
mod presets;
mod preview_access;
mod preview_files;
mod preview_grants;
mod preview_host;
mod preview_ipc;
mod preview_serve;
mod preview_text;
mod preview_watch;
mod pty;
mod pty_branch;
mod resources;
mod serve;
mod shot;
mod state;
mod stdio;
mod strings;
mod tray;
mod update_host;
mod vault;
mod webstorage;
mod window;
mod worktree;
mod wtcopy;
mod shim;

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use tauri::ipc::CapabilityBuilder;
use tauri::{AppHandle, Manager, RunEvent};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use crate::config::ShellConfig;
use crate::serve::{Serve, ServeError};
use crate::state::{Shell, MAIN};

fn context() -> tauri::Context<tauri::Wry> {
    tauri::generate_context!()
}

fn fail(message: String) -> ! {
    eprintln!("[shell] {message}");
    let _ = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            app.dialog().message(message.clone()).title("Orrery").kind(MessageDialogKind::Error).blocking_show();
            app.handle().exit(1);
            Ok(())
        })
        .run(context());
    std::process::exit(1)
}

fn capability(app: &AppHandle, origin: &str) -> tauri::Result<()> {
    let cap = CapabilityBuilder::new("app")
        .remote(format!("{origin}/*"))
        .windows([MAIN, window::COMPACT])
        .permission("core:default")
        .permission("core:window:allow-start-dragging")
        .permission("core:window:allow-internal-toggle-maximize")
        .permission("allow-ccm");
    app.add_capability(cap)?;
    let guests = CapabilityBuilder::new("guest")
        .webview("guest-*")
        .remote("http://*".into())
        .remote("http://*:*".into())
        .remote("https://*".into())
        .remote("https://*:*".into())
        .permission("allow-guest");
    app.add_capability(guests)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cfg = ShellConfig::from_env(&args).unwrap_or_else(|e| fail(e));
    let appearance = appearance::Store::open(cfg.data_dir.join("shell-appearance.json"));
    let t = strings::strings(appearance.locale());
    let _ = std::fs::create_dir_all(&cfg.data_dir);
    log::install(&cfg.data_dir, config::APP_VERSION);
    let desk = desk::boot(&desk::env_for(&cfg.data_dir, config::APP_VERSION));
    let serve = match Serve::start(cfg.serve.clone()) {
        Ok(s) => s,
        Err(ServeError::PortInUse) => fail(t.port_in_use.replace("{port}", &cfg.port.to_string())),
        Err(e) => fail(t.start_failed.replace("{reason}", &format!("{e:?}"))),
    };
    let origin = cfg.origin();
    let shell = Shell {
        port: cfg.port,
        key: cfg.key.clone(),
        origin: origin.clone(),
        data_dir: cfg.data_dir.clone(),
        webview_dir: cfg.webview_dir.clone(),
        init_script: webstorage::with_import(&cfg.data_dir, &origin, window::init_script(&origin)),
        pty: pty::PtyHost::new(cfg.core_bin.clone(), origin.clone(), config::APP_VERSION.into(), cfg.data_dir.join("pty.log")),
        serve: Mutex::new(Some(serve)),
        serve_cfg: cfg.serve.clone(),
        look: Mutex::new(appearance),
        replies: Mutex::new(HashMap::new()),
        link_times: Mutex::new(Vec::new()),
        quitting: AtomicBool::new(false),
        stopped: AtomicBool::new(false),
        before_stop: Mutex::new(None),
    };
    let hidden = cfg.hidden;
    let first_link = links::link_in(&args);
    let mut builder = tauri::Builder::default();
    if !config::e2e_isolated() {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| links::second_instance(app, argv)));
    }
    let app = builder
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(shell)
        .manage(desk)
        .manage(guest::Guests::default())
        .invoke_handler(tauri::generate_handler![ccm::ccm, guest_ipc::guest])
        .on_window_event(window::on_window_event)
        .setup(move |app| {
            let handle = app.handle().clone();
            capability(&handle, &origin)?;
            desk::wire(&handle);
            let stopping = handle.clone();
            *state::lock(&handle.state::<Shell>().before_stop) = Some(Box::new(move || lifecycle::clean(&stopping)));
            window::create_main(&handle, hidden).map_err(std::io::Error::other)?;
            tray::create(&handle)?;
            let links_app = handle.clone();
            app.deep_link().on_open_url(move |event| {
                for url in event.urls() {
                    links::handle(&links_app, url.as_str());
                }
            });
            if let Some(link) = first_link.clone() {
                links::handle(&handle, &link);
            }
            Ok(())
        })
        .build(context())
        .unwrap_or_else(|e| fail(e.to_string()));
    app.run(|app, event| match event {
        RunEvent::ExitRequested { api, code, .. } => {
            if code.is_none() && !app.state::<Shell>().quitting.load(std::sync::atomic::Ordering::SeqCst) {
                api.prevent_exit();
            }
        }
        RunEvent::Exit => app.state::<Shell>().shutdown(),
        _ => {}
    });
}
