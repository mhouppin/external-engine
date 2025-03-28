use std::{
    process::{Command, Stdio},
    sync::Arc,
};

use clap::Parser;
use ksni::{
    menu::{Disposition, MenuItem, StandardItem},
    Icon, Status, Tray, TrayService,
};
use remote_uci::{ExternalWorkerOpts, Opt};
use tokio::sync::Notify;

fn xdg_open(url: &str) {
    match Command::new("xdg-open")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(_) => log::info!("opened {}", url),
        Err(err) => log::error!("failed to open {}: {}", url, err),
    }
}

struct RemoteUciTray {
    shutdown: Arc<Notify>,
    spec: ExternalWorkerOpts,
}

impl Tray for RemoteUciTray {
    fn id(&self) -> String {
        "remote-uci-applet".into()
    }

    fn title(&self) -> String {
        "External Lichess Engine".into()
    }

    fn status(&self) -> Status {
        Status::Passive
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![Icon {
            width: 32,
            height: 32,
            data: include_bytes!("../lichess-favicon-32-invert.argb32").to_vec(),
        }]
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: "Connect".into(),
                activate: Box::new(|tray: &mut RemoteUciTray| {
                    xdg_open(&tray.spec.registration_url())
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "License".into(),
                disposition: Disposition::Informative,
                activate: Box::new(|_: &mut RemoteUciTray| {
                    xdg_open("https://github.com/lichess-org/external-engine/blob/main/COPYING.md")
                }),
                // icon_name: "help-about".into(),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Shutdown".into(),
                // icon_name: "application-exit".into(),
                activate: Box::new(|tray: &mut RemoteUciTray| tray.shutdown.notify_one()),
                ..Default::default()
            }
            .into(),
        ]
    }
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::new()
            .filter("REMOTE_UCI_LOG")
            .default_filter_or("info")
            .write_style("REMOTE_UCI_LOG_STYLE"),
    )
    .format_target(false)
    .format_module_path(false)
    .init();

    let opt = Opt::parse();

    let (spec, server) = remote_uci::make_server(opt).await;
    log::info!("registration url: {}", spec.registration_url());

    let shutdown = Arc::new(Notify::new());
    TrayService::new(RemoteUciTray {
        shutdown: Arc::clone(&shutdown),
        spec,
    })
    .spawn();

    server
        .with_graceful_shutdown(shutdown.notified())
        .await
        .expect("bind");
}
