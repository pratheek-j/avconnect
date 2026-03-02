//! Backend services — AWS EC2 discovery, config persistence, SSH execution,
//! SFTP connectivity, and host registry merging.

mod aws;
mod config;
mod host_registry;
pub mod sftp;
mod ssh;

pub use config::{
    add_managed_host, config_to_draft,
    default_profile, delete_managed_host, draft_to_config, first_run_needed,
    import_hosts_from_ssh_config, load_config, save_config,
};
pub use host_registry::load_hosts;
pub use ssh::connect as ssh_connect;
