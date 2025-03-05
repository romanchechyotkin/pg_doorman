use super::{get_reporter, Reporter};
use iota::iota;
use std::sync::atomic::*;
use std::sync::Arc;
use tokio::time::Instant;

iota! {
    pub const CLIENT_STATE_IDLE: u8 = 10 << iota;
        , CLIENT_STATE_ACTIVE
        , CLIENT_STATE_WAITING
}

iota! {
    pub const CLIENT_WAIT_IDLE: u8 = 20 << iota;
        , CLIENT_WAIT_READ
        , CLIENT_WAIT_WRITE
}

#[derive(Clone)]
/// Information we keep track of which can be queried by SHOW CLIENTS
pub struct ClientStats {
    /// A random integer assigned to the client and used by stats to track the client
    client_id: i32,

    /// Data associated with the client, not writable, only set when we construct the ClientStat
    application_name: String,
    username: String,
    pool_name: String,
    ipaddr: String,
    connect_time: Instant,
    use_tls: bool,

    reporter: Reporter,

    /// Total time spent waiting for a connection from pool, measures in microseconds
    pub total_wait_time: Arc<AtomicU64>,

    /// Maximum time spent waiting for a connection from pool, measures in microseconds
    pub max_wait_time: Arc<AtomicU64>,

    /// Current state of the client
    pub state: Arc<AtomicU8>,

    /// Current wait status of the client
    pub wait: Arc<AtomicU8>,

    /// Number of transactions executed by this client
    pub transaction_count: Arc<AtomicU64>,

    /// Number of queries executed by this client
    pub query_count: Arc<AtomicU64>,

    /// Number of errors made by this client
    pub error_count: Arc<AtomicU64>,
}

impl Default for ClientStats {
    fn default() -> Self {
        ClientStats {
            client_id: 0,
            connect_time: Instant::now(),
            application_name: String::new(),
            username: String::new(),
            pool_name: String::new(),
            ipaddr: String::new(),
            total_wait_time: Arc::new(AtomicU64::new(0)),
            max_wait_time: Arc::new(AtomicU64::new(0)),
            state: Arc::new(AtomicU8::new(CLIENT_STATE_IDLE)),
            wait: Arc::new(AtomicU8::new(CLIENT_WAIT_IDLE)),
            transaction_count: Arc::new(AtomicU64::new(0)),
            query_count: Arc::new(AtomicU64::new(0)),
            error_count: Arc::new(AtomicU64::new(0)),
            reporter: get_reporter(),
            use_tls: false,
        }
    }
}

impl ClientStats {
    pub fn new(
        client_id: i32,
        application_name: &str,
        username: &str,
        pool_name: &str,
        ipaddr: &str,
        connect_time: Instant,
        use_tls: bool,
    ) -> Self {
        Self {
            client_id,
            connect_time,
            application_name: application_name.to_string(),
            username: username.to_string(),
            pool_name: pool_name.to_string(),
            ipaddr: ipaddr.to_string(),
            use_tls,
            ..Default::default()
        }
    }

    /// Reports a client is disconnecting from the pooler and
    /// update metrics on the corresponding pool.
    #[inline(always)]
    pub fn disconnect(&self) {
        self.reporter.client_disconnecting(self.client_id);
    }

    /// Register a client with the stats system. The stats system uses client_id
    /// to track and aggregate statistics from all source that relate to that client
    pub fn register(&self, stats: Arc<ClientStats>) {
        self.reporter.client_register(self.client_id, stats);
        self.state.store(CLIENT_STATE_IDLE, Ordering::Relaxed);
    }

    /// Reports a client is done querying the server and is no longer assigned a server connection, and we're reading from client.
    #[inline(always)]
    pub fn idle_read(&self) {
        self.state.store(CLIENT_STATE_IDLE, Ordering::Relaxed);
        self.wait.store(CLIENT_WAIT_READ, Ordering::Relaxed);
    }

    /// Reports a client is done querying the server and is no longer assigned a server connection, but we're writing to client.
    #[inline(always)]
    pub fn idle_write(&self) {
        self.state.store(CLIENT_STATE_IDLE, Ordering::Relaxed);
        self.wait.store(CLIENT_WAIT_WRITE, Ordering::Relaxed);
    }

    /// Reports a client is waiting for a connection.
    #[inline(always)]
    pub fn waiting(&self) {
        self.state.store(CLIENT_STATE_WAITING, Ordering::Relaxed);
        self.wait.store(CLIENT_WAIT_IDLE, Ordering::Relaxed);
    }

    /// Reports a client is done waiting for a connection, and we're reading from it.
    #[inline(always)]
    pub fn active_read(&self) {
        self.state.store(CLIENT_STATE_ACTIVE, Ordering::Relaxed);
        self.wait.store(CLIENT_WAIT_READ, Ordering::Relaxed);
    }

    /// Reports a client is done waiting for a connection, and we're writing to it.
    #[inline(always)]
    pub fn active_write(&self) {
        self.state.store(CLIENT_STATE_ACTIVE, Ordering::Relaxed);
        self.wait.store(CLIENT_WAIT_WRITE, Ordering::Relaxed);
    }

    /// Reports a client is done waiting for a connection, and wait response from server.
    #[inline(always)]
    pub fn active_idle(&self) {
        self.state.store(CLIENT_STATE_ACTIVE, Ordering::Relaxed);
        self.wait.store(CLIENT_WAIT_IDLE, Ordering::Relaxed);
    }

    pub fn state_to_string(&self) -> String {
        match self.state.load(Ordering::Relaxed) {
            CLIENT_STATE_WAITING => "waiting".to_string(),
            CLIENT_STATE_IDLE => "idle".to_string(),
            CLIENT_STATE_ACTIVE => "active".to_string(),
            _ => "unknown".to_string(),
        }
    }

    pub fn wait_to_string(&self) -> String {
        match self.wait.load(Ordering::Relaxed) {
            CLIENT_WAIT_IDLE => "idle".to_string(),
            CLIENT_WAIT_WRITE => "write".to_string(),
            CLIENT_WAIT_READ => "read".to_string(),
            _ => "unknown".to_string(),
        }
    }

    /// Reports a client has failed to obtain a connection from a connection pool
    #[inline(always)]
    pub fn checkout_error(&self) {
        self.state.store(CLIENT_STATE_IDLE, Ordering::Relaxed);
        self.wait.store(CLIENT_WAIT_IDLE, Ordering::Relaxed);
    }

    /// Report a query executed by a client against a server
    #[inline(always)]
    pub fn query(&self) {
        self.query_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Report a transaction executed by a client a server
    /// we report each individual queries outside a transaction as a transaction
    /// We only count the initial BEGIN as a transaction, all queries within do not
    /// count as transactions
    #[inline(always)]
    pub fn transaction(&self) {
        self.transaction_count.fetch_add(1, Ordering::Relaxed);
    }

    // Helper methods for show clients
    #[inline(always)]
    pub fn connect_time(&self) -> Instant {
        self.connect_time
    }

    #[inline(always)]
    pub fn client_id(&self) -> i32 {
        self.client_id
    }

    #[inline(always)]
    pub fn application_name(&self) -> String {
        self.application_name.clone()
    }

    #[inline(always)]
    pub fn tls(&self) -> bool {
        self.use_tls
    }

    #[inline(always)]
    pub fn username(&self) -> String {
        self.username.clone()
    }

    #[inline(always)]
    pub fn pool_name(&self) -> String {
        self.pool_name.clone()
    }

    #[inline(always)]
    pub fn ipaddr(&self) -> String {
        self.ipaddr.clone()
    }
}
