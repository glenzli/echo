//! Live recovery coordinator for durable background analysis.
//!
//! Catalog owns state and error policy. This owner only waits for a verified
//! Infer Runtime contract to become reachable, then returns current transient
//! failures to Echo's durable queue with bounded backoff.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use echo_catalog::{Catalog, automatic_analysis_recovery_count, requeue_automatic_analysis};

use crate::{InferRuntimeClient, InferRuntimeConfig};

const IDLE_POLL: Duration = Duration::from_secs(1);
const STOP_POLL: Duration = Duration::from_millis(200);
const RETRY_DELAYS: [Duration; 5] = [
    Duration::from_secs(2),
    Duration::from_secs(8),
    Duration::from_secs(20),
    Duration::from_secs(40),
    Duration::from_mins(1),
];

pub(crate) fn spawn(
    catalog: Arc<Catalog>,
    config: InferRuntimeConfig,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || recovery_loop(&catalog, config, &stop))
}

fn recovery_loop(catalog: &Catalog, config: InferRuntimeConfig, stop: &AtomicBool) {
    let runtime = InferRuntimeClient::new(config);
    let mut schedule = RecoverySchedule::default();
    while !stop.load(Ordering::Acquire) {
        let count = catalog
            .with_transaction(|transaction| {
                automatic_analysis_recovery_count(
                    transaction,
                    crate::CONTEXTUAL_SCHEMA_VERSION,
                    crate::CONTEXTUAL_JOB_REVISION,
                    crate::LONG_AUDIO_PLAN_VERSION,
                )
            })
            .unwrap_or(0);
        if count == 0 {
            schedule.reset();
            interruptible_sleep(stop, IDLE_POLL);
            continue;
        }
        if let Some(delay) = schedule.remaining() {
            interruptible_sleep(stop, delay.min(IDLE_POLL));
            continue;
        }
        if runtime.contract_version().is_ok() {
            let now = crate::util::now_millis();
            let _ = catalog.with_transaction(|transaction| {
                requeue_automatic_analysis(
                    transaction,
                    crate::CONTEXTUAL_SCHEMA_VERSION,
                    crate::CONTEXTUAL_JOB_REVISION,
                    crate::LONG_AUDIO_PLAN_VERSION,
                    now,
                )
            });
        }
        schedule.defer();
    }
}

fn interruptible_sleep(stop: &AtomicBool, duration: Duration) {
    let deadline = Instant::now() + duration;
    while !stop.load(Ordering::Acquire) {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return;
        }
        thread::sleep(remaining.min(STOP_POLL));
    }
}

#[derive(Debug, Default)]
struct RecoverySchedule {
    attempt: usize,
    next_probe: Option<Instant>,
}

impl RecoverySchedule {
    fn remaining(&self) -> Option<Duration> {
        self.next_probe
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .filter(|duration| !duration.is_zero())
    }

    fn defer(&mut self) {
        let delay = RETRY_DELAYS[self.attempt.min(RETRY_DELAYS.len() - 1)];
        self.attempt = self.attempt.saturating_add(1);
        self.next_probe = Some(Instant::now() + delay);
    }

    fn reset(&mut self) {
        self.attempt = 0;
        self.next_probe = None;
    }
}

#[cfg(test)]
mod tests;
