use log::{debug, error, warn};

use crate::config::get_config;
use crate::{config::PoolMode, messages::DataType, pool::PoolIdentifierVirtual};
use std::collections::HashMap;
use std::sync::atomic::*;

use crate::pool::{get_all_pools, StatsPoolIdentifier};
use crate::stats::client::{CLIENT_STATE_ACTIVE, CLIENT_STATE_IDLE, CLIENT_STATE_WAITING};
use crate::stats::percenitle::percentile_of_sorted;
use crate::stats::server::{SERVER_STATE_ACTIVE, SERVER_STATE_IDLE, SERVER_STATE_LOGIN};

#[derive(Debug, Clone)]
/// A struct that holds information about a Pool .
pub struct PoolStats {
    pub identifier: PoolIdentifierVirtual,
    pub mode: PoolMode,
    pub cl_idle: u64,
    pub cl_active: u64,
    pub cl_waiting: u64,
    pub cl_cancel_req: u64,
    pub sv_active: u64,
    pub sv_idle: u64,
    pub sv_used: u64,
    pub sv_login: u64,
    pub maxwait: u64,
    pub avg_xact_count: u64,
    pub avg_query_count: u64,
    pub avg_wait_time: u64,
    pub avg_wait_time_vp_ms: Vec<f64>,
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub xact_time: u64,
    pub query_time: u64,
    pub wait_time: u64,
    pub errors: u64,
    // percentiles.
    queries: Vec<u64>,
    xact: Vec<u64>,
    percentile_updated: bool,
    pub xact_percentile: Percentile,
    pub query_percentile: Percentile,
    // show stats.
    total_xact_count: u64,
    total_query_count: u64,
    total_received: u64,
    total_sent: u64,
    total_xact_time_microseconds: u64,
    total_query_time_microseconds: u64,
    avg_recv: u64,
    avg_sent: u64,
    avg_xact_time_microsecons: u64,
    avg_query_time_microseconds: u64,
}

#[derive(Debug, Clone)]
/// A struct that holds information about a Pool and aggregated info about a clients.
pub struct PoolClientStats {
    pub pool_stats: PoolStats,
}

impl PoolClientStats {
    pub fn new(
        identifier: PoolIdentifierVirtual,
        mode: PoolMode,
        queries: Vec<u64>,
        xact: Vec<u64>,
    ) -> Self {
        PoolClientStats {
            pool_stats: PoolStats::new(identifier, mode, queries, xact),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Percentile {
    pub p99: u64,
    pub p95: u64,
    pub p90: u64,
    pub p50: u64,
}

impl PoolStats {
    pub fn new(
        identifier: PoolIdentifierVirtual,
        mode: PoolMode,
        queries: Vec<u64>,
        xact: Vec<u64>,
    ) -> Self {
        PoolStats {
            identifier,
            mode,
            cl_idle: 0,
            cl_active: 0,
            cl_waiting: 0,
            cl_cancel_req: 0,
            sv_active: 0,
            sv_idle: 0,
            sv_used: 0,
            sv_login: 0,
            maxwait: 0,
            avg_query_count: 0,
            avg_xact_count: 0,
            avg_wait_time: 0,
            avg_wait_time_vp_ms: Vec::new(),
            bytes_received: 0,
            bytes_sent: 0,
            xact_time: 0,
            query_time: 0,
            wait_time: 0,
            errors: 0,
            xact,
            queries,
            percentile_updated: false,
            xact_percentile: Percentile {
                p99: 0,
                p95: 0,
                p90: 0,
                p50: 0,
            },
            query_percentile: Percentile {
                p99: 0,
                p95: 0,
                p90: 0,
                p50: 0,
            },
            total_xact_count: 0,
            total_query_count: 0,
            total_received: 0,
            total_sent: 0,
            total_xact_time_microseconds: 0,
            total_query_time_microseconds: 0,
            avg_recv: 0,
            avg_sent: 0,
            avg_xact_time_microsecons: 0,
            avg_query_time_microseconds: 0,
        }
    }

    pub fn construct_pool_lookup() -> HashMap<StatsPoolIdentifier, PoolStats> {
        let mut virtual_map: HashMap<PoolIdentifierVirtual, PoolStats> = HashMap::new();
        let client_map = super::get_client_stats();
        let server_map = super::get_server_stats();

        for (identifier, pool) in get_all_pools() {
            let address = pool.address().stats.clone();
            let mut queries = Vec::new();
            {
                let lock = address.query_times_us.lock();
                queries.extend(lock.iter())
            }
            let mut xact = Vec::new();
            {
                let lock = address.xact_times_us.lock();
                xact.extend(lock.iter())
            }
            let mut current =
                PoolStats::new(identifier.clone(), pool.settings.pool_mode, queries, xact);
            current.avg_xact_count = address.averages.xact_count.load(Ordering::Relaxed);
            current.avg_query_count = address.averages.query_count.load(Ordering::Relaxed);
            current.bytes_received = address.total.bytes_received.load(Ordering::Relaxed);
            current.bytes_sent = address.total.bytes_sent.load(Ordering::Relaxed);
            current.xact_time = address.total.xact_time_microseconds.load(Ordering::Relaxed);
            current.query_time = address
                .total
                .query_time_microseconds
                .load(Ordering::Relaxed);
            current.wait_time = address.total.wait_time.load(Ordering::Relaxed);
            current.errors = address.averages.errors.load(Ordering::Relaxed);
            // show stats;
            current.total_xact_count = address.total.xact_count.load(Ordering::Relaxed);
            current.total_query_count = address.total.query_count.load(Ordering::Relaxed);
            current.total_received = address.total.bytes_received.load(Ordering::Relaxed);
            current.total_sent = address.total.bytes_sent.load(Ordering::Relaxed);
            current.total_xact_time_microseconds =
                address.total.xact_time_microseconds.load(Ordering::Relaxed);
            current.total_query_time_microseconds = address
                .total
                .query_time_microseconds
                .load(Ordering::Relaxed);
            current.avg_recv = address.averages.bytes_received.load(Ordering::Relaxed);
            current.avg_sent = address.averages.bytes_sent.load(Ordering::Relaxed);
            current.avg_xact_time_microsecons = address
                .averages
                .xact_time_microseconds
                .load(Ordering::Relaxed);
            current.avg_query_time_microseconds = address
                .averages
                .query_time_microseconds
                .load(Ordering::Relaxed);
            if current.avg_xact_count > 0 {
                current.avg_wait_time =
                    address.averages.wait_time.load(Ordering::Relaxed) / current.avg_xact_count;
                current
                    .avg_wait_time_vp_ms
                    .push(current.avg_wait_time as f64 / 1_000f64);
            }
            virtual_map.insert(identifier.clone(), current);
        }

        for virtual_pool_id in 0..get_config().general.virtual_pool_count {
            for client in client_map.values() {
                match virtual_map.get_mut(&PoolIdentifierVirtual {
                    db: client.pool_name(),
                    user: client.username(),
                    virtual_pool_id,
                }) {
                    Some(pool_stats) => {
                        match client.state.load(Ordering::Relaxed) {
                            CLIENT_STATE_ACTIVE => pool_stats.cl_active += 1,
                            CLIENT_STATE_IDLE => pool_stats.cl_idle += 1,
                            CLIENT_STATE_WAITING => pool_stats.cl_waiting += 1,
                            _ => error!("unknown client state"),
                        };
                        let max_wait = client.max_wait_time.load(Ordering::Relaxed);
                        pool_stats.maxwait = std::cmp::max(pool_stats.maxwait, max_wait);
                    }
                    None => debug!("Client from an obsolete pool"),
                }
            }

            for server in server_map.values() {
                match virtual_map.get_mut(&PoolIdentifierVirtual {
                    db: server.pool_name(),
                    user: server.username(),
                    virtual_pool_id,
                }) {
                    Some(pool_stats) => match server.state.load(Ordering::Relaxed) {
                        SERVER_STATE_ACTIVE => pool_stats.sv_active += 1,
                        SERVER_STATE_IDLE => pool_stats.sv_idle += 1,
                        SERVER_STATE_LOGIN => pool_stats.sv_login += 1,
                        _ => error!("unknown server state"),
                    },
                    None => warn!("Server from an obsolete pool"),
                }
            }
        }

        // Задача объеденить данные по виртуальным идентификаторам.
        let mut map: HashMap<StatsPoolIdentifier, PoolStats> = HashMap::new();
        for (id, virtual_pool_stat) in virtual_map {
            let db_name = id.db.clone();
            let user_name = id.user.clone();
            let stats_pool_id = StatsPoolIdentifier {
                db: db_name.clone(),
                user: user_name.clone(),
            };
            let mut exists_in_map = true;
            let queries = virtual_pool_stat.queries.clone();
            let xact = virtual_pool_stat.xact.clone();
            // обновляем значение.
            match map.get_mut(&stats_pool_id) {
                Some(current) => {
                    current.xact.extend(xact.clone());
                    current.queries.extend(queries.clone());
                    current.avg_query_count += virtual_pool_stat.avg_query_count;
                    current.avg_xact_count += virtual_pool_stat.avg_xact_count;
                    current.bytes_received += virtual_pool_stat.bytes_received;
                    current.bytes_sent += virtual_pool_stat.bytes_sent;
                    current.xact_time += virtual_pool_stat.xact_time;
                    current.query_time += virtual_pool_stat.query_time;
                    current.wait_time += virtual_pool_stat.wait_time;
                    current.errors += virtual_pool_stat.errors;
                    if virtual_pool_stat.avg_xact_count > 0 {
                        current.avg_wait_time =
                            (current.avg_wait_time + virtual_pool_stat.avg_wait_time) / 2;
                        current
                            .avg_wait_time_vp_ms
                            .push(virtual_pool_stat.avg_wait_time as f64 / 1_000f64);
                    }
                    // show stats;
                    current.total_xact_count += virtual_pool_stat.total_xact_count;
                    current.total_query_count += virtual_pool_stat.total_query_count;
                    current.total_received += virtual_pool_stat.total_received;
                    current.total_sent += virtual_pool_stat.total_sent;
                    current.total_xact_time_microseconds +=
                        virtual_pool_stat.total_xact_time_microseconds;
                    current.total_query_time_microseconds +=
                        virtual_pool_stat.total_query_time_microseconds;
                    current.avg_recv += virtual_pool_stat.avg_recv;
                    current.avg_sent += virtual_pool_stat.avg_sent;
                    // TODO: учитывать вес по xact count.
                    current.avg_xact_time_microsecons = (current.avg_xact_time_microsecons
                        + virtual_pool_stat.avg_xact_time_microsecons)
                        / 2;
                    current.avg_query_time_microseconds = (current.avg_query_time_microseconds
                        + virtual_pool_stat.avg_query_time_microseconds)
                        / 2;
                }
                None => exists_in_map = false,
            }
            if !exists_in_map {
                // вставляем значение.
                map.insert(stats_pool_id, virtual_pool_stat);
            }
        }

        // обновляем перцентили.
        for (id, _stat) in get_all_pools() {
            if let Some(value) = map.get_mut(&StatsPoolIdentifier {
                db: id.db.clone(),
                user: id.user.clone(),
            }) {
                if value.percentile_updated {
                    continue;
                }
                {
                    let times = value.queries.as_mut_slice();
                    times.sort();
                    value.query_percentile.p50 = percentile_of_sorted(times, 50.0);
                    value.query_percentile.p90 = percentile_of_sorted(times, 90.0);
                    value.query_percentile.p95 = percentile_of_sorted(times, 95.0);
                    value.query_percentile.p99 = percentile_of_sorted(times, 99.0);
                }
                {
                    let times = value.xact.as_mut_slice();
                    times.sort();
                    value.xact_percentile.p50 = percentile_of_sorted(times, 50.0);
                    value.xact_percentile.p90 = percentile_of_sorted(times, 90.0);
                    value.xact_percentile.p95 = percentile_of_sorted(times, 95.0);
                    value.xact_percentile.p99 = percentile_of_sorted(times, 99.0);
                }
                value.percentile_updated = true
            }
        }

        map
    }

    pub fn generate_show_pools_header() -> Vec<(&'static str, DataType)> {
        vec![
            ("database", DataType::Text),
            ("user", DataType::Text),
            ("pool_mode", DataType::Text),
            ("cl_idle", DataType::Numeric),
            ("cl_active", DataType::Numeric),
            ("cl_waiting", DataType::Numeric),
            ("cl_cancel_req", DataType::Numeric),
            ("sv_active", DataType::Numeric),
            ("sv_idle", DataType::Numeric),
            ("sv_used", DataType::Numeric),
            ("sv_login", DataType::Numeric),
            ("maxwait", DataType::Numeric),
            ("maxwait_us", DataType::Numeric),
        ]
    }

    // generate_extended_header like odyssey.
    pub fn generate_show_pools_extended_header() -> Vec<(&'static str, DataType)> {
        vec![
            ("database", DataType::Text),
            ("user", DataType::Text),
            ("cl_active", DataType::Numeric),
            ("cl_waiting", DataType::Numeric),
            ("sv_active", DataType::Numeric),
            ("sv_idle", DataType::Numeric),
            ("sv_used", DataType::Numeric),
            ("sv_login", DataType::Numeric),
            ("maxwait", DataType::Numeric),
            ("maxwait_us", DataType::Numeric),
            ("pool_mode", DataType::Text),
            ("bytes_recieved", DataType::Numeric),
            ("bytes_sent", DataType::Numeric),
            ("query_0.99", DataType::Numeric),
            ("transaction_0.99", DataType::Numeric),
            ("query_0.95", DataType::Numeric),
            ("transaction_0.95", DataType::Numeric),
            ("query_0.5", DataType::Numeric),
            ("transaction_0.5", DataType::Numeric),
        ]
    }

    pub fn generate_show_pools_extended_row(&self) -> Vec<String> {
        vec![
            self.identifier.db.clone(),
            self.identifier.user.clone(),
            self.cl_active.to_string(),
            self.cl_waiting.to_string(),
            self.sv_active.to_string(),
            self.sv_idle.to_string(),
            self.sv_used.to_string(),
            self.sv_login.to_string(),
            (self.maxwait as f64 / 1_000_000f64).to_string(),
            (self.maxwait % 1_000_000).to_string(),
            self.mode.to_string(),
            self.bytes_received.to_string(),
            self.bytes_sent.to_string(),
            self.query_percentile.p99.to_string(),
            self.xact_percentile.p99.to_string(),
            self.query_percentile.p95.to_string(),
            self.xact_percentile.p95.to_string(),
            self.query_percentile.p50.to_string(),
            self.xact_percentile.p50.to_string(),
        ]
    }

    pub fn generate_show_pools_row(&self) -> Vec<String> {
        vec![
            self.identifier.db.clone(),
            self.identifier.user.clone(),
            self.mode.to_string(),
            self.cl_idle.to_string(),
            self.cl_active.to_string(),
            self.cl_waiting.to_string(),
            self.cl_cancel_req.to_string(),
            self.sv_active.to_string(),
            self.sv_idle.to_string(),
            self.sv_used.to_string(),
            self.sv_login.to_string(),
            (self.maxwait / 1_000_000).to_string(),
            (self.maxwait % 1_000_000).to_string(),
        ]
    }

    pub fn generate_show_stats_header() -> Vec<(&'static str, DataType)> {
        vec![
            ("database", DataType::Text),
            ("user", DataType::Text),
            ("total_xact_count", DataType::Numeric),
            ("total_query_count", DataType::Numeric),
            ("total_received", DataType::Numeric),
            ("total_sent", DataType::Numeric),
            ("total_xact_time", DataType::Numeric),
            ("total_query_time", DataType::Numeric),
            ("total_wait_time", DataType::Numeric),
            ("total_errors", DataType::Numeric),
            ("avg_xact_count", DataType::Numeric),
            ("avg_query_count", DataType::Numeric),
            ("avg_recv", DataType::Numeric),
            ("avg_sent", DataType::Numeric),
            ("avg_errors", DataType::Numeric),
            ("avg_xact_time", DataType::Numeric),
            ("avg_query_time", DataType::Numeric),
            ("avg_wait_time", DataType::Numeric),
        ]
    }

    pub fn generate_show_stats_row(&self) -> Vec<String> {
        vec![
            self.identifier.db.clone(),
            self.identifier.user.clone(),
            self.total_xact_count.to_string(),
            self.total_query_count.to_string(),
            self.total_received.to_string(),
            self.total_sent.to_string(),
            self.total_xact_time_microseconds.to_string(),
            self.total_query_time_microseconds.to_string(),
            self.wait_time.to_string(),
            self.errors.to_string(),
            self.avg_xact_count.to_string(),
            self.avg_query_count.to_string(),
            self.avg_recv.to_string(),
            self.avg_sent.to_string(),
            self.errors.to_string(),
            self.avg_xact_time_microsecons.to_string(),
            self.avg_query_time_microseconds.to_string(),
            self.avg_wait_time.to_string(),
        ]
    }
}

impl IntoIterator for PoolStats {
    type Item = (String, u64);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        vec![
            ("cl_idle".to_string(), self.cl_idle),
            ("cl_active".to_string(), self.cl_active),
            ("cl_waiting".to_string(), self.cl_waiting),
            ("cl_cancel_req".to_string(), self.cl_cancel_req),
            ("sv_active".to_string(), self.sv_active),
            ("sv_idle".to_string(), self.sv_idle),
            ("sv_used".to_string(), self.sv_used),
            ("sv_login".to_string(), self.sv_login),
            ("maxwait".to_string(), self.maxwait / 1_000_000),
            ("maxwait_us".to_string(), self.maxwait % 1_000_000),
        ]
        .into_iter()
    }
}
