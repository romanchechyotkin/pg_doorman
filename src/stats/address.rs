use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::atomic::*;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct AddressStatFields {
    pub xact_count: Arc<AtomicU64>,
    pub query_count: Arc<AtomicU64>,
    pub bytes_received: Arc<AtomicU64>,
    pub bytes_sent: Arc<AtomicU64>,
    pub xact_time_microseconds: Arc<AtomicU64>,
    pub query_time_microseconds: Arc<AtomicU64>,
    pub wait_time: Arc<AtomicU64>,
    pub errors: Arc<AtomicU64>,
}

/// Internal address stats
#[derive(Debug, Clone, Default)]
pub struct AddressStats {
    pub total: AddressStatFields,

    pub current: AddressStatFields,

    pub averages: AddressStatFields,

    // Determines if the averages have been updated since the last time they were reported
    pub averages_updated: Arc<AtomicBool>,

    pub xact_times_us: Arc<Mutex<VecDeque<u64>>>,
    pub query_times_us: Arc<Mutex<VecDeque<u64>>>,
}

const QUERY_EXCEPTED_TIMES_CAPACITY: usize = 8 * 1024;
const QUERY_MAX_TIMES_CAPACITY: usize = 2 * QUERY_EXCEPTED_TIMES_CAPACITY;

impl IntoIterator for AddressStats {
    type Item = (String, f64);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        vec![
            (
                "total_xact_count".to_string(),
                self.total.xact_count.load(Ordering::Relaxed) as f64,
            ),
            (
                "total_query_count".to_string(),
                self.total.query_count.load(Ordering::Relaxed) as f64,
            ),
            (
                "total_received".to_string(),
                self.total.bytes_received.load(Ordering::Relaxed) as f64,
            ),
            (
                "total_sent".to_string(),
                self.total.bytes_sent.load(Ordering::Relaxed) as f64,
            ),
            (
                "total_xact_time".to_string(),
                self.total.xact_time_microseconds.load(Ordering::Relaxed) as f64 / 1_000f64,
            ),
            (
                "total_query_time".to_string(),
                self.total.query_time_microseconds.load(Ordering::Relaxed) as f64 / 1_000f64,
            ),
            (
                "total_wait_time".to_string(),
                self.total.wait_time.load(Ordering::Relaxed) as f64,
            ),
            (
                "total_errors".to_string(),
                self.total.errors.load(Ordering::Relaxed) as f64,
            ),
            (
                "avg_xact_count".to_string(),
                self.averages.xact_count.load(Ordering::Relaxed) as f64,
            ),
            (
                "avg_query_count".to_string(),
                self.averages.query_count.load(Ordering::Relaxed) as f64,
            ),
            (
                "avg_recv".to_string(),
                self.averages.bytes_received.load(Ordering::Relaxed) as f64,
            ),
            (
                "avg_sent".to_string(),
                self.averages.bytes_sent.load(Ordering::Relaxed) as f64,
            ),
            (
                "avg_errors".to_string(),
                self.averages.errors.load(Ordering::Relaxed) as f64,
            ),
            (
                "avg_xact_time".to_string(),
                self.averages.xact_time_microseconds.load(Ordering::Relaxed) as f64,
            ),
            (
                "avg_query_time".to_string(),
                self.averages
                    .query_time_microseconds
                    .load(Ordering::Relaxed) as f64,
            ),
            (
                "avg_wait_time".to_string(),
                self.averages.wait_time.load(Ordering::Relaxed) as f64,
            ),
        ]
        .into_iter()
    }
}

impl AddressStats {
    #[inline(always)]
    pub fn xact_count_add(&self) {
        self.total.xact_count.fetch_add(1, Ordering::Relaxed);
        self.current.xact_count.fetch_add(1, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn query_count_add(&self) {
        self.total.query_count.fetch_add(1, Ordering::Relaxed);
        self.current.query_count.fetch_add(1, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn bytes_received_add(&self, bytes: u64) {
        self.total
            .bytes_received
            .fetch_add(bytes, Ordering::Relaxed);
        self.current
            .bytes_received
            .fetch_add(bytes, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn bytes_sent_add(&self, bytes: u64) {
        self.total.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
        self.current.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn xact_time_add(&self, microseconds: u64) {
        if microseconds == 0 {
            return;
        }
        self.total
            .xact_time_microseconds
            .fetch_add(microseconds, Ordering::Relaxed);
        self.current
            .xact_time_microseconds
            .fetch_add(microseconds, Ordering::Relaxed);
        if let Some(mut t) = self.xact_times_us.try_lock() {
            t.push_front(microseconds); // VecDeque always grow x2.
            if t.len() > QUERY_MAX_TIMES_CAPACITY {
                t.truncate(QUERY_EXCEPTED_TIMES_CAPACITY);
            }
        };
    }

    #[inline(always)]
    pub fn query_time_add_microseconds(&self, microseconds: u64) {
        self.total
            .query_time_microseconds
            .fetch_add(microseconds, Ordering::Relaxed);
        self.current
            .query_time_microseconds
            .fetch_add(microseconds, Ordering::Relaxed);
        if let Some(mut t) = self.query_times_us.try_lock() {
            t.push_front(microseconds); // VecDeque always grow x2.
            if t.len() > QUERY_MAX_TIMES_CAPACITY {
                t.truncate(QUERY_EXCEPTED_TIMES_CAPACITY);
            }
        };
    }

    #[inline(always)]
    pub fn wait_time_add(&self, time: u64) {
        self.total.wait_time.fetch_add(time, Ordering::Relaxed);
        self.current.wait_time.fetch_add(time, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn error(&self) {
        self.total.errors.fetch_add(1, Ordering::Relaxed);
        self.current.errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn update_averages(&self) {
        let stat_period_per_second = crate::stats::STAT_PERIOD / 1_000;

        // xact_count
        let current_xact_count = self.current.xact_count.load(Ordering::Relaxed);
        let current_xact_time = self.current.xact_time_microseconds.load(Ordering::Relaxed);
        self.averages.xact_count.store(
            current_xact_count / stat_period_per_second,
            Ordering::Relaxed,
        );
        if current_xact_count == 0 {
            self.averages
                .xact_time_microseconds
                .store(0, Ordering::Relaxed);
        } else {
            self.averages
                .xact_time_microseconds
                .store(current_xact_time / current_xact_count, Ordering::Relaxed);
        }

        // query_count
        let current_query_count = self.current.query_count.load(Ordering::Relaxed);
        let current_query_time = self.current.query_time_microseconds.load(Ordering::Relaxed);
        self.averages.query_count.store(
            current_query_count / stat_period_per_second,
            Ordering::Relaxed,
        );
        if current_query_count == 0 {
            self.averages
                .query_time_microseconds
                .store(0, Ordering::Relaxed);
        } else {
            self.averages
                .query_time_microseconds
                .store(current_query_time / current_query_count, Ordering::Relaxed);
        }

        // bytes_received
        let current_bytes_received = self.current.bytes_received.load(Ordering::Relaxed);
        self.averages.bytes_received.store(
            current_bytes_received / stat_period_per_second,
            Ordering::Relaxed,
        );

        // bytes_sent
        let current_bytes_sent = self.current.bytes_sent.load(Ordering::Relaxed);
        self.averages.bytes_sent.store(
            current_bytes_sent / stat_period_per_second,
            Ordering::Relaxed,
        );

        // wait_time
        let current_wait_time = self.current.wait_time.load(Ordering::Relaxed);
        self.averages.wait_time.store(
            current_wait_time / stat_period_per_second,
            Ordering::Relaxed,
        );

        // errors
        let current_errors = self.current.errors.load(Ordering::Relaxed);
        self.averages
            .errors
            .store(current_errors / stat_period_per_second, Ordering::Relaxed);
    }

    pub fn reset_current_counts(&self) {
        self.current.xact_count.store(0, Ordering::Relaxed);
        self.current
            .xact_time_microseconds
            .store(0, Ordering::Relaxed);
        self.current.query_count.store(0, Ordering::Relaxed);
        self.current
            .query_time_microseconds
            .store(0, Ordering::Relaxed);
        self.current.bytes_received.store(0, Ordering::Relaxed);
        self.current.bytes_sent.store(0, Ordering::Relaxed);
        self.current.wait_time.store(0, Ordering::Relaxed);
        self.current.errors.store(0, Ordering::Relaxed);
    }

    pub fn populate_row(&self, row: &mut Vec<String>) {
        for (_key, value) in self.clone() {
            row.push(value.to_string());
        }
    }
}
