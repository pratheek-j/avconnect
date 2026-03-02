//! EC2 instance discovery — queries AWS for running instances and maps
//! them to the application's Host model.

use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_ec2::types::InstanceStateName;
use aws_sdk_ec2::Client;
use std::collections::HashMap;

use crate::state::{Host, HostSource, InstanceState};

pub async fn fetch_hosts(
    profile: &str,
    region: &str,
    access_key_id: Option<&str>,
    secret_access_key: Option<&str>,
) -> Result<Vec<Host>> {
    let region_name = region.to_string();
    let region_cfg = aws_config::Region::new(region_name.clone());
    let config = if let (Some(ak), Some(sk)) = (access_key_id, secret_access_key) {
        std::env::set_var("AWS_ACCESS_KEY_ID", ak);
        std::env::set_var("AWS_SECRET_ACCESS_KEY", sk);
        aws_config::defaults(BehaviorVersion::latest())
            .region(region_cfg.clone())
            .load()
            .await
    } else {
        aws_config::defaults(BehaviorVersion::latest())
            .profile_name(profile)
            .region(region_cfg.clone())
            .load()
            .await
    };
    let client = Client::new(&config);
    let out = client
        .describe_instances()
        .send()
        .await
        .context("describe_instances")?;

    let mut hosts = Vec::new();
    for res in out.reservations().iter().flat_map(|r| r.instances()) {
        let instance_id = res.instance_id().unwrap_or("").to_string();
        let state = res
            .state()
            .and_then(|s| s.name())
            .map(instance_state_to_model)
            .unwrap_or(InstanceState::Stopped);
        let tags: HashMap<String, String> = res
            .tags()
            .iter()
            .filter_map(|t| {
                let k = t.key()?;
                let v = t.value()?;
                Some((k.to_string(), v.to_string()))
            })
            .collect();
        let name = tags
            .get("Name")
            .cloned()
            .unwrap_or_else(|| instance_id.clone());
        let username = tags.get("User").cloned().unwrap_or_else(|| "ubuntu".to_string());
        let address = res
            .public_ip_address()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "---".to_string());

        hosts.push(Host {
            name,
            address,
            username,
            port: 22,
            key_path: None,
            state: Some(state),
            source: HostSource::Aws {
                instance_id,
                region: region_name.clone(),
            },
        });
    }
    Ok(hosts)
}

fn instance_state_to_model(s: &InstanceStateName) -> InstanceState {
    use InstanceStateName::*;
    match s {
        Pending => InstanceState::Pending,
        Running => InstanceState::Running,
        ShuttingDown => InstanceState::ShuttingDown,
        Stopped => InstanceState::Stopped,
        Stopping => InstanceState::Stopping,
        Terminated => InstanceState::Terminated,
        _ => InstanceState::Stopped,
    }
}
