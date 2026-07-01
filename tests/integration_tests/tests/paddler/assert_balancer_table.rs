use anyhow::Result;
use anyhow::anyhow;

use crate::agent_response::AgentsResponse;

fn assert_fields(table_fields: Vec<Option<&String>>, peer_fields: Vec<String>) {
    for (index, table_field) in table_fields.iter().enumerate() {
        if let Some(field) = table_field {
            assert_eq!(**field, peer_fields[index])
        }
    }
}

pub fn assert_balancer_table(
    table: &cucumber::gherkin::Table,
    agents_response: &AgentsResponse,
) -> Result<()> {
    let headers = &table.rows[0];

    for row in table.rows.iter().skip(1) {
        let agent_name = row
            .first()
            .ok_or_else(|| anyhow!("Missing agent name in table row"))?;

        let peer = agents_response
            .agents
            .iter()
            .find(|p| &p.status.agent_name == agent_name)
            .ok_or_else(|| anyhow!("Agent {} not found in response", agent_name))?;

        let mut table_fields = Vec::new();
        let mut peer_fields = Vec::new();

        for (col_idx, header) in headers.iter().enumerate() {
            match header.as_str() {
                "agent" => {
                    table_fields.push(row.get(col_idx));
                    peer_fields.push(peer.status.agent_name.clone());
                }
                "is_connect_error" => {
                    table_fields.push(row.get(col_idx));
                    peer_fields.push(
                        peer.status
                            .is_connect_error
                            .map(|v| v.to_string())
                            .unwrap_or("None".to_string()),
                    );
                }
                _ => continue,
            }
        }

        assert_fields(table_fields, peer_fields);
    }

    Ok(())
}
