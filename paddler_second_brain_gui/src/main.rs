mod agent_monitor_service;
mod agent_running_data;
mod agent_status_monitor_service;
mod detect_network_interfaces;
mod font;
mod home_data;
mod join_cluster_config_data;
mod message;
mod model_preset;
mod network_interface_address;
mod running_cluster_data;
mod screen;
mod screen_current;
mod second_brain;
mod start_agent;
mod start_agent_services;
mod start_balancer;
mod start_balancer_services;
mod start_cluster_config_data;
mod style_agent_container;
mod style_button_disconnect;
mod style_button_primary;
mod style_card_container;
mod style_download_progress_bar;
mod style_field_container;
mod style_field_pick_list;
mod style_field_pick_list_menu;
mod style_field_text_input;
mod variables;
mod view_agent_card;
mod view_agent_running;
mod view_home;
mod view_join_cluster_config;
mod view_running_cluster;
mod view_start_cluster_config;

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
