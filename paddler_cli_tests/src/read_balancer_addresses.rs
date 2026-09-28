use paddler_balancer::balancer_addresses::BalancerAddresses;
use tokio::io::AsyncBufReadExt as _;
use tokio::io::AsyncRead;
use tokio::io::BufReader;

use crate::subprocess_cluster_error::SubprocessClusterError;

pub async fn read_balancer_addresses<TReader>(
    reader: TReader,
) -> Result<BalancerAddresses, SubprocessClusterError>
where
    TReader: AsyncRead + Unpin,
{
    let announcement = BufReader::new(reader)
        .lines()
        .next_line()
        .await
        .map_err(|source| SubprocessClusterError::AnnouncementUnreadable { source })?
        .ok_or(SubprocessClusterError::StdoutClosedBeforeAnnouncement)?;

    serde_json::from_str(&announcement).map_err(|source| {
        SubprocessClusterError::AnnouncementInvalid {
            announcement,
            source,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::read_balancer_addresses;
    use crate::subprocess_cluster_error::SubprocessClusterError;

    #[tokio::test]
    async fn reports_stdout_closed_before_the_announcement() {
        let read_error = read_balancer_addresses(&b""[..])
            .await
            .expect_err("an empty stdout must not yield balancer addresses");

        assert!(matches!(
            read_error,
            SubprocessClusterError::StdoutClosedBeforeAnnouncement
        ));
    }

    #[tokio::test]
    async fn rejects_an_announcement_that_is_not_balancer_addresses() {
        let read_error = read_balancer_addresses(&b"{}\n"[..])
            .await
            .expect_err("an object without addresses must be rejected");

        assert!(matches!(
            read_error,
            SubprocessClusterError::AnnouncementInvalid { announcement, .. } if announcement == "{}"
        ));
    }

    #[tokio::test]
    async fn reports_an_unreadable_announcement() {
        let read_error = read_balancer_addresses(&[0xff, b'\n'][..])
            .await
            .expect_err("a line that is not UTF-8 must not be readable");

        assert!(matches!(
            read_error,
            SubprocessClusterError::AnnouncementUnreadable { .. }
        ));
    }
}
