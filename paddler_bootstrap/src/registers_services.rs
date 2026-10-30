use trzcina::ServiceManager;

pub trait RegistersServices {
    fn register_services(self, service_manager: &mut ServiceManager);
}
