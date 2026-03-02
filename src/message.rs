//! All state-transition messages that flow through the reducer.
//! Keyboard events and async service results are mapped into these
//! variants before being dispatched to `reduce()`.

use crate::state::{FileNode, Host, SshProfile, SetupStep};

#[derive(Clone, Debug)]
pub enum AppMessage {
    MenuSelect(usize),
    MenuConfirm,
    GoBack,
    Quit,
    DismissError,

    HostsLoaded(Vec<Host>),
    HostsLoadFailed(String),
    RefreshHosts,
    SearchQueryChanged(String),
    HostListMoveUp,
    HostListMoveDown,
    OpenNewHostModal,

    OpenKeySelect { host: Host, profiles: Vec<SshProfile> },
    OpenSftpKeySelect { host: Host, profiles: Vec<SshProfile> },
    KeySelectMove(usize),

    SftpConnected {
        host: Host,
        profile: SshProfile,
        root_entries: Vec<FileNode>,
    },
    SftpBrowserMoveUp,
    SftpBrowserMoveDown,
    SftpBrowserToggleSelection,
    SftpConfirmSelection(crate::state::SftpSource),
    SftpStartTransfer,

    ConfigFieldEdited { field: String, value: String },
    ConfigFocusNext,
    ConfigFocusPrev,
    ConfigEditStart,
    ConfigEditStop,
    ConfigSaved,

    FirstRunInputChanged(String),
    FirstRunEditStart,
    FirstRunEditStop,
    SetupStepCompleted(SetupStep),
}
