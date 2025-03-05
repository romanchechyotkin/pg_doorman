use super::AddressStats;
use super::{get_reporter, Reporter};
use crate::config::Address;
use iota::iota;
use parking_lot::RwLock;
use std::sync::atomic::*;
use std::sync::Arc;
use tokio::time::Instant;

iota! {
    pub const SERVER_STATE_LOGIN: u8 = 30 << iota;
        , SERVER_STATE_ACTIVE
        , SERVER_STATE_IDLE
}

iota! {
    pub const SERVER_WAIT_IDLE: u8 = 40 << iota;
        , SERVER_WAIT_READ
        , SERVER_WAIT_WRITE
}

/// Information we keep track of which can be queried by SHOW SERVERS
#[derive(Debug, Clone)]
pub struct ServerStats {
    /// A random integer assigned to the server and used by stats to track the server
    server_id: i32,
    process_id: Arc<AtomicI32>,

    /// Context information, only to be read
    address: Address,
    connect_time: Instant,

    reporter: Reporter,

    /// Data
    pub application_name: Arc<RwLock<String>>,
    pub state: Arc<AtomicU8>,
    pub wait: Arc<AtomicU8>,
    pub bytes_sent: Arc<AtomicU64>,
    pub bytes_received: Arc<AtomicU64>,
    pub transaction_count: Arc<AtomicU64>,
    pub query_count: Arc<AtomicU64>,
    pub error_count: Arc<AtomicU64>,
    pub prepared_hit_count: Arc<AtomicU64>,
    pub prepared_miss_count: Arc<AtomicU64>,
    pub prepared_cache_size: Arc<AtomicU64>,
}

impl Default for ServerStats {
    fn default() -> Self {
        ServerStats {
            server_id: 0,
            process_id: Arc::new(AtomicI32::new(0)),
            application_name: Arc::new(RwLock::new(String::new())),
            address: Address::default(),
            connect_time: Instant::now(),
            state: Arc::new(AtomicU8::new(SERVER_STATE_LOGIN)),
            wait: Arc::new(AtomicU8::new(SERVER_WAIT_IDLE)),
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            transaction_count: Arc::new(AtomicU64::new(0)),
            query_count: Arc::new(AtomicU64::new(0)),
            error_count: Arc::new(AtomicU64::new(0)),
            reporter: get_reporter(),
            prepared_hit_count: Arc::new(AtomicU64::new(0)),
            prepared_miss_count: Arc::new(AtomicU64::new(0)),
            prepared_cache_size: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl ServerStats {
    pub fn new(address: Address, connect_time: Instant) -> Self {
        Self {
            address,
            connect_time,
            server_id: rand::random::<i32>(),
            ..Default::default()
        }
    }

    pub fn server_id(&self) -> i32 {
        self.server_id
    }

    pub fn process_id(&self) -> i32 {
        self.process_id.load(Ordering::Relaxed)
    }

    /// Register a server connection with the stats system. The stats system uses server_id
    /// to track and aggregate statistics from all source that relate to that server
    // Delegates to reporter
    pub fn register(&self, stats: Arc<ServerStats>) {
        self.reporter.server_register(self.server_id, stats);
        self.login();
    }

    /// Reports a server connection is no longer assigned to a client
    /// and is available for the next client to pick it up
    #[inline(always)]
    pub fn idle(&self, microseconds: u64) {
        self.address.stats.xact_time_add(microseconds);
        self.state.store(SERVER_STATE_IDLE, Ordering::Relaxed);
    }

    pub fn update_process_id(&self, id: i32) {
        self.process_id.store(id, Ordering::Relaxed);
    }

    /// just write server xact time.
    #[inline(always)]
    pub fn add_xact_time_and_idle(&self, microseconds: u64) {
        self.state.store(SERVER_STATE_IDLE, Ordering::Relaxed);
        self.address.stats.xact_time_add(microseconds);
    }

    /// Reports a server connection is disconnecting from the pooler.
    /// Also updates metrics on the pool regarding server usage.
    #[inline(always)]
    pub fn disconnect(&self) {
        self.reporter.server_disconnecting(self.server_id);
    }

    /// Reports a server connection is attempting to login.
    pub fn login(&self) {
        self.state.store(SERVER_STATE_LOGIN, Ordering::Relaxed);
        self.set_undefined_application();
    }

    /// Reports a server connection has been assigned to a client.
    pub fn active(&self, application_name: String) {
        self.state.store(SERVER_STATE_ACTIVE, Ordering::Relaxed);
        self.set_application(application_name);
    }

    /// Reading from the server connection.
    #[inline]
    pub fn wait_reading(&self) {
        self.wait.store(SERVER_WAIT_READ, Ordering::Relaxed);
    }

    /// Writing to the server connection.
    #[inline]
    pub fn wait_writing(&self) {
        self.wait.store(SERVER_WAIT_WRITE, Ordering::Relaxed);
    }

    /// Idle to the server connection.
    #[inline]
    pub fn wait_idle(&self) {
        self.wait.store(SERVER_WAIT_IDLE, Ordering::Relaxed);
    }

    pub fn state_to_string(&self) -> String {
        match self.state.load(Ordering::Relaxed) {
            SERVER_STATE_ACTIVE => "active".to_string(),
            SERVER_STATE_IDLE => "idle".to_string(),
            SERVER_STATE_LOGIN => "login".to_string(),
            _ => "unknown".to_string(),
        }
    }

    pub fn wait_to_string(&self) -> String {
        match self.wait.load(Ordering::Relaxed) {
            SERVER_WAIT_IDLE => "idle".to_string(),
            SERVER_WAIT_READ => "read".to_string(),
            SERVER_WAIT_WRITE => "write".to_string(),
            _ => "unknown".to_string(),
        }
    }

    pub fn address_stats(&self) -> Arc<AddressStats> {
        self.address.stats.clone()
    }

    pub fn check_address_stat_average_is_updated_status(&self) -> bool {
        self.address.stats.averages_updated.load(Ordering::Relaxed)
    }

    pub fn set_address_stat_average_is_updated_status(&self, is_checked: bool) {
        self.address
            .stats
            .averages_updated
            .store(is_checked, Ordering::Relaxed);
    }

    // Helper methods for show_servers
    pub fn pool_name(&self) -> String {
        self.address.pool_name.clone()
    }

    pub fn username(&self) -> String {
        self.address.username.clone()
    }

    pub fn address_name(&self) -> String {
        self.address.name()
    }

    pub fn connect_time(&self) -> Instant {
        self.connect_time
    }

    fn set_application(&self, name: String) {
        let mut application_name = self.application_name.write();
        *application_name = name;
    }

    #[inline(always)]
    fn set_undefined_application(&self) {
        self.set_application(String::from("Undefined"))
    }

    #[inline(always)]
    pub fn checkout_time(&self, microseconds: u64, application_name: String) {
        // Update server stats and address aggregation stats
        self.set_application(application_name);
        self.address.stats.wait_time_add(microseconds);
    }

    /// Report a query executed by a client against a server
    #[inline(always)]
    pub fn query(&self, microseconds: u64, application_name: &str) {
        self.set_application(application_name.to_string());
        self.address.stats.query_count_add();
        self.address.stats.query_time_add_microseconds(microseconds);
        self.query_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Report a transaction executed by a client a server
    /// we report each individual queries outside a transaction as a transaction
    /// We only count the initial BEGIN as a transaction, all queries within do not
    /// count as transactions
    #[inline(always)]
    pub fn transaction(&self, application_name: &str) {
        self.set_application(application_name.to_string());

        self.transaction_count.fetch_add(1, Ordering::Relaxed);
        self.address.stats.xact_count_add();
    }

    /// Report data sent to a server
    #[inline(always)]
    pub fn data_sent(&self, amount_bytes: usize) {
        self.bytes_sent
            .fetch_add(amount_bytes as u64, Ordering::Relaxed);
        self.address.stats.bytes_sent_add(amount_bytes as u64);
    }

    /// Report data received from a server
    #[inline(always)]
    pub fn data_received(&self, amount_bytes: usize) {
        self.bytes_received
            .fetch_add(amount_bytes as u64, Ordering::Relaxed);
        self.address.stats.bytes_received_add(amount_bytes as u64);
    }

    /// Report a prepared statement that already exists on the server.
    #[inline(always)]
    pub fn prepared_cache_hit(&self) {
        self.prepared_hit_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Report a prepared statement that does not exist on the server yet.
    #[inline(always)]
    pub fn prepared_cache_miss(&self) {
        self.prepared_miss_count.fetch_add(1, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn prepared_cache_add(&self) {
        self.prepared_cache_size.fetch_add(1, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn prepared_cache_remove(&self) {
        self.prepared_cache_size.fetch_sub(1, Ordering::Relaxed);
    }
}
