use trzcina::service_manager::ServiceManager;

pub trait RegistersServices {
    fn register_services(self, service_manager: &mut ServiceManager);
}
