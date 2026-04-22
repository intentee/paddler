mod agent_running_data;
mod agent_running_handler;
mod app;
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

use app::App;
use iced::Size;
use iced::Theme;

fn main() -> iced::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

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
