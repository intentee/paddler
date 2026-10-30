use std::future::Future;

pub trait ShutsDown {
    fn shut_down(self) -> impl Future<Output = ()> + Send;
}
