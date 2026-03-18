mod agent_monitor_service;
mod agent_running_data;
mod agent_status_monitor_service;
mod detect_network_interfaces;
mod join_cluster_config_data;
mod message;
mod model_preset;
mod network_interface_address;
mod network_monitor_service;
mod running_cluster_data;
mod screen;
mod screen_current;
mod second_brain;
mod start_agent;
mod start_agent_services;
mod start_balancer;
mod start_balancer_services;
mod start_cluster_config_data;
mod view_agent_running;
mod view_home;
mod view_join_cluster_config;
mod view_running_cluster;
mod view_start_cluster_config;

use second_brain::SecondBrain;

fn main() -> iced::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    iced::application(SecondBrain::new, SecondBrain::update, SecondBrain::view)
        .subscription(SecondBrain::subscription)
        .run()
}
