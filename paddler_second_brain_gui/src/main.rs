mod agent_running_data;
mod detect_network_interfaces;
mod home_data;
mod join_cluster_config_data;
mod message;
mod model_preset;
mod network_interface_address;
mod running_cluster_data;
mod screen;
mod screen_current;
mod second_brain;
mod start_cluster_config_data;
mod ui;

use iced::Size;
use second_brain::SecondBrain;

fn main() -> iced::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    iced::application(SecondBrain::new, SecondBrain::update, SecondBrain::view)
        .font(include_bytes!(
            "../../resources/fonts/JetBrainsMono-Regular.ttf"
        ))
        .font(include_bytes!(
            "../../resources/fonts/JetBrainsMono-Bold.ttf"
        ))
        .window_size(Size::new(800.0, 800.0))
        .subscription(SecondBrain::subscription)
        .run()
}
