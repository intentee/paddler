mod agent_running_data;
mod agent_running_handler;
mod app;
mod auto_cluster_config;
mod current_screen;
mod detect_network_interfaces;
mod home_data;
mod home_handler;
mod join_cluster_config_data;
mod join_cluster_config_handler;
mod message;
mod model_preset;
mod network_interface_address;
mod running_cluster_data;
mod running_cluster_handler;
mod running_cluster_snapshot;
#[expect(unsafe_code, reason = "statum macros generate link_section statics")]
mod screen;
mod start_cluster_config_data;
mod start_cluster_config_handler;
mod ui;

use std::net::SocketAddr;
use std::net::TcpListener;

use app::App;
#[cfg(feature = "web_admin_panel")]
use esbuild_metafile::instance::initialize_instance;
use iced::Size;
use iced::Theme;
use log::info;

use crate::auto_cluster_config::AutoClusterConfig;
use crate::auto_cluster_config::install_auto_cluster_config;

#[cfg(feature = "web_admin_panel")]
const ESBUILD_META_CONTENTS: &str = include_str!("../../esbuild-meta.json");

fn pick_free_loopback_addr() -> anyhow::Result<SocketAddr> {
    let probe = TcpListener::bind("127.0.0.1:0")?;
    let addr = probe.local_addr()?;

    drop(probe);

    Ok(addr)
}

#[expect(
    clippy::expect_used,
    reason = "auto-cluster loopback bind is only used by the integration-test harness and is unrecoverable"
)]
fn main() -> iced::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    #[cfg(feature = "web_admin_panel")]
    initialize_instance(ESBUILD_META_CONTENTS);

    if std::env::var_os("PADDLER_GUI_AUTO_CLUSTER").is_some() {
        let management_addr = pick_free_loopback_addr()
            .expect("failed to pick management loopback addr for auto-cluster");
        let inference_addr = pick_free_loopback_addr()
            .expect("failed to pick inference loopback addr for auto-cluster");

        info!("paddler_gui: auto-cluster management={management_addr} inference={inference_addr}");

        install_auto_cluster_config(AutoClusterConfig {
            inference_addr,
            management_addr,
        });
    }

    info!("paddler_gui: ready");

    iced::application(App::new, App::update, App::view)
        .font(include_bytes!(
            "../../resources/fonts/JetBrainsMono-Regular.ttf"
        ))
        .font(include_bytes!(
            "../../resources/fonts/JetBrainsMono-Bold.ttf"
        ))
        .theme(Theme::Light)
        .window_size(Size::new(800.0, 800.0))
        .subscription(App::subscription)
        .run()
}
