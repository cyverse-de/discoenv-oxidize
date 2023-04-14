#[derive(Debug, Clone)]
pub struct HandlerConfiguration {
    pub append_user_domain: bool,
    pub user_domain: String,
    pub do_auth: bool,
}
