/// Shared application state for hooks and runners.
/// Extend this struct with whatever runtime fields you need to share.
#[derive(Debug, Default, Clone)]
pub struct AppState {
    pub remote: Option<RemoteInfo>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct RemoteInfo {
    pub user: String,
    pub host: String,
    pub port: u16,
    pub password: String,
}
