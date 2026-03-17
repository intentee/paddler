mod message;
mod running_cluster_data;
mod screen;
mod screen_current;
mod second_brain;
mod start_balancer;
mod start_balancer_services;
mod start_cluster_config_data;
mod starting_cluster_data;
mod view_home;
mod view_running_cluster;
mod view_start_cluster_config;
mod view_starting_cluster;
mod view_stopping_cluster;

use second_brain::SecondBrain;

fn main() -> iced::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    iced::application(SecondBrain::new, SecondBrain::update, SecondBrain::view).run()
}
