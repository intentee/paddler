use serde_json::from_str;
use tokio::io::AsyncBufReadExt as _;
use tokio::io::AsyncRead;
use tokio::io::BufReader;

use paddler_balancer::balancer_addresses::BalancerAddresses;

use crate::subprocess_cluster_error::SubprocessClusterError;

pub async fn read_balancer_addresses<TReader>(
    reader: TReader,
) -> Result<Option<BalancerAddresses>, SubprocessClusterError>
where
    TReader: AsyncRead + Unpin,
{
    let Some(announcement) = BufReader::new(reader)
        .lines()
        .next_line()
        .await
        .map_err(|source| SubprocessClusterError::AnnouncementUnreadable { source })?
    else {
        return Ok(None);
    };

    from_str(&announcement).map(Some).map_err(|source| {
        SubprocessClusterError::AnnouncementInvalid {
            announcement,
            source,
        }
    })
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;
    use std::io;
    use std::mem::discriminant;

    use super::read_balancer_addresses;
    use crate::subprocess_cluster_error::SubprocessClusterError;

    #[tokio::test]
    async fn yields_no_addresses_when_stdout_closes_before_the_announcement() {
        assert_eq!(read_balancer_addresses(&b""[..]).await.ok(), Some(None));
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

        assert_eq!(
            discriminant(&read_error),
            discriminant(&SubprocessClusterError::AnnouncementUnreadable {
                source: io::Error::from(io::ErrorKind::InvalidData)
            })
        );
        assert_eq!(
            read_error
                .source()
                .and_then(|source| source.downcast_ref::<io::Error>())
                .map(io::Error::kind),
            Some(io::ErrorKind::InvalidData)
        );
    }
}
