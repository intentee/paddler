mod balancer_status;
mod message;
mod second_brain;
mod start_balancer;
mod start_balancer_services;

use second_brain::SecondBrain;

fn main() -> iced::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    iced::application(SecondBrain::new, SecondBrain::update, SecondBrain::view).run()
}
