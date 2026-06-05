use nix::sys::resource::Resource;
use nix::sys::resource::getrlimit;

use crate::balancer_http_server::BalancerHttpServer;
use crate::file_descriptor_limit_error::FileDescriptorLimitError;

/// File descriptors each actix worker's tokio runtime keeps open: measured at 4 on macOS (three
/// kqueue descriptors and one unix-domain socket per worker) with `lsof` against running balancers
/// configured at 20, 22, and 38 workers, where the per-worker slope is exact across all three.
/// Conservative on epoll platforms such as Linux, whose runtimes use fewer descriptors and whose
/// default `RLIMIT_NOFILE` is higher.
const FDS_PER_WORKER: u64 = 4;

/// File descriptors each active HTTP server keeps open beyond its workers: measured at 3 on macOS
/// (one bound listener and two kqueue descriptors) by the same `lsof` fit.
const FDS_PER_SERVER: u64 = 3;

/// File descriptors the balancer holds open independent of its HTTP servers: stdio, the main actix
/// `System` runtime, shutdown-signal handling, the binary image, and the working directory.
/// Measured at 12 on macOS by the same `lsof` fit.
const BASE_PROCESS_FDS: u64 = 12;

fn required_file_descriptors(active_servers: &[BalancerHttpServer]) -> u64 {
    let worker_descriptors: u64 = active_servers
        .iter()
        .map(|active_server| active_server.worker_count() as u64 * FDS_PER_WORKER)
        .sum();
    let server_descriptors = active_servers.len() as u64 * FDS_PER_SERVER;

    worker_descriptors + server_descriptors + BASE_PROCESS_FDS
}

const fn evaluate_file_descriptor_limit(
    soft_limit: u64,
    required: u64,
) -> Result<(), FileDescriptorLimitError> {
    if soft_limit < required {
        Err(FileDescriptorLimitError::InsufficientDescriptors {
            soft_limit,
            required,
        })
    } else {
        Ok(())
    }
}

pub fn ensure_file_descriptor_limit(
    active_servers: &[BalancerHttpServer],
) -> Result<(), FileDescriptorLimitError> {
    let required = required_file_descriptors(active_servers);
    let soft_limit = getrlimit(Resource::RLIMIT_NOFILE)
        .map_err(|errno| FileDescriptorLimitError::UnableToReadLimit {
            message: errno.to_string(),
        })?
        .0;

    evaluate_file_descriptor_limit(soft_limit, required)
}

#[cfg(test)]
mod tests {
    use super::BalancerHttpServer;
    use super::FileDescriptorLimitError;
    use super::ensure_file_descriptor_limit;
    use super::evaluate_file_descriptor_limit;
    use super::required_file_descriptors;

    fn all_servers() -> [BalancerHttpServer; 4] {
        [
            BalancerHttpServer::Inference,
            BalancerHttpServer::Management,
            BalancerHttpServer::OpenAI,
            BalancerHttpServer::WebAdminPanel,
        ]
    }

    #[test]
    fn errors_when_soft_limit_is_below_requirement() {
        let required = required_file_descriptors(&all_servers());

        match evaluate_file_descriptor_limit(required - 1, required) {
            Err(FileDescriptorLimitError::InsufficientDescriptors {
                soft_limit,
                required: reported_required,
            }) => {
                assert_eq!(soft_limit, required - 1);
                assert_eq!(reported_required, required);
            }
            other => panic!("expected InsufficientDescriptors, got {other:?}"),
        }
    }

    #[test]
    fn insufficient_descriptors_message_states_the_numbers_and_the_remedy() {
        let message = FileDescriptorLimitError::InsufficientDescriptors {
            soft_limit: 64,
            required: 176,
        }
        .to_string();

        assert!(message.contains("64"));
        assert!(message.contains("176"));
        assert!(message.contains("ulimit -n 176"));
    }

    #[test]
    fn succeeds_when_soft_limit_meets_requirement() {
        let required = required_file_descriptors(&all_servers());

        assert!(evaluate_file_descriptor_limit(required, required).is_ok());
    }

    #[test]
    fn requirement_grows_with_each_active_server() {
        let core_servers = [
            BalancerHttpServer::Inference,
            BalancerHttpServer::Management,
        ];

        assert!(
            required_file_descriptors(&all_servers()) > required_file_descriptors(&core_servers)
        );
    }

    #[test]
    fn ensure_succeeds_under_the_test_process_limit() {
        assert!(ensure_file_descriptor_limit(&all_servers()).is_ok());
    }
}
