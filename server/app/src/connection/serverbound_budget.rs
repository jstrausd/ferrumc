//! Per-connection serverbound packet budget: a token bucket that gates how fast a
//! client may send play frames, dropping a sustained flood.
//!
//! Wraps the networking [`PacketBudget`] with the per-connection over-budget tally
//! the play loop publishes into telemetry. Lives in the connection task (which is
//! allowed a non-deterministic [`Instant`] clock) and is never consulted from the
//! deterministic sim/session tick path — it is a small, pure struct so the policy
//! is unit-testable without a live socket.

use std::time::{Duration, Instant};

use ferrumc_net::{DisconnectReason, PacketBudget};

/// The result of charging a frame against the unchanged connection quota.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BudgetAdmission {
    /// One token was spent before packet effects.
    Admitted,
    /// Keep this single frame until a real token refills.
    Deferred(Instant),
}

/// Fatal exhaustion retains a typed leave reason.
#[derive(Debug)]
pub(super) struct ServerboundBudgetExceeded;

impl std::fmt::Display for ServerboundBudgetExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("serverbound packet budget exceeded (BudgetExceeded)")
    }
}

impl std::error::Error for ServerboundBudgetExceeded {}

/// Per-connection quota with a finite, rate-paced recovery episode.
pub(super) struct ServerboundBudget {
    /// The token bucket charged one token per serverbound frame.
    budget: PacketBudget,
    /// Frames classified over budget for this connection (published into the
    /// per-player network telemetry). Saturates rather than wraps.
    over_budget: u64,
    established: bool,
    recovery_until: Option<Instant>,
}

impl ServerboundBudget {
    /// Builds a full bucket as of `now` with the configured sustained
    /// `rate_per_sec` and `burst`.
    pub(super) fn new(now: Instant, rate_per_sec: f64, burst: f64) -> Self {
        Self {
            budget: PacketBudget::new(now, rate_per_sec, burst),
            over_budget: 0,
            established: false,
            recovery_until: None,
        }
    }

    /// Charges one serverbound frame as of `now`.
    ///
    /// Returns `Ok(())` while the peer stays within budget. On the first
    /// over-budget frame it tallies the event and returns the
    /// [`DisconnectReason::BudgetExceeded`] the caller drops the connection with —
    /// a deterministic, documented response to a flood rather than an unbounded
    /// backlog of unhandled frames.
    pub(super) fn admit(&mut self, now: Instant) -> Result<(), DisconnectReason> {
        self.budget.admit_frame(now).inspect_err(|_reason| {
            self.over_budget = self.over_budget.saturating_add(1);
        })
    }

    /// Initial login-handoff traffic remains strict; only established Play paces.
    pub(super) fn establish_play(&mut self) {
        self.established = true;
    }

    /// Keeps the same token quota, with one recovery episode no longer than a
    /// burst refill period or two seconds. Only real half-burst replenishment
    /// resets an episode, so continuous flooding cannot extend it.
    pub(super) fn admit_or_defer(
        &mut self,
        now: Instant,
        quota_retry: bool,
    ) -> Result<BudgetAdmission, DisconnectReason> {
        if !self.established {
            self.admit(now)?;
            return Ok(BudgetAdmission::Admitted);
        }
        self.budget.refill(now);
        if !quota_retry && self.budget.available_tokens() >= self.budget.burst() * 0.5 {
            self.recovery_until = None;
        }
        if self.budget.available_tokens() >= 1.0 {
            self.admit(now)?;
            return Ok(BudgetAdmission::Admitted);
        }
        let rate = self.budget.rate_per_sec();
        let recovery = Duration::try_from_secs_f64(self.budget.burst() / rate)
            .unwrap_or(Duration::ZERO)
            .min(Duration::from_secs(2));
        let refill = Duration::try_from_secs_f64((1.0 - self.budget.available_tokens()) / rate)
            .unwrap_or(Duration::ZERO)
            .saturating_add(Duration::from_nanos(1));
        if recovery.is_zero() || refill.is_zero() {
            return self.admit(now).map(|()| BudgetAdmission::Admitted);
        }
        let until = *self.recovery_until.get_or_insert_with(|| now + recovery);
        if now >= until {
            return self.admit(now).map(|()| BudgetAdmission::Admitted);
        }
        Ok(BudgetAdmission::Deferred((now + refill).min(until)))
    }

    /// The number of serverbound frames classified over budget for this
    /// connection, for the per-player network telemetry snapshot.
    pub(super) fn over_budget(&self) -> u64 {
        self.over_budget
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use ferrumc_net::DisconnectReason;

    use super::ServerboundBudget;

    #[test]
    fn normal_traffic_stays_within_budget() {
        let now = Instant::now();
        let mut budget = ServerboundBudget::new(now, 300.0, 600.0);
        // A realistic burst of movement/actions well under the 600 burst passes
        // without ever tripping the budget.
        for _ in 0..200 {
            assert_eq!(budget.admit(now), Ok(()));
        }
        assert_eq!(budget.over_budget(), 0);
    }

    #[test]
    fn a_flood_past_the_burst_is_disconnected_and_counted() {
        let now = Instant::now();
        // A small bucket charged at a single instant: the burst passes, then the
        // flood is dropped deterministically and every over-budget frame is tallied.
        let mut budget = ServerboundBudget::new(now, 300.0, 5.0);
        for _ in 0..5 {
            assert_eq!(budget.admit(now), Ok(()));
        }
        assert_eq!(budget.admit(now), Err(DisconnectReason::BudgetExceeded));
        assert_eq!(budget.admit(now), Err(DisconnectReason::BudgetExceeded));
        assert_eq!(budget.over_budget(), 2);
    }

    #[test]
    fn the_bucket_refills_so_a_paced_client_is_never_dropped() {
        let t0 = Instant::now();
        let mut budget = ServerboundBudget::new(t0, 300.0, 1.0);
        assert_eq!(budget.admit(t0), Ok(()));
        assert_eq!(budget.admit(t0), Err(DisconnectReason::BudgetExceeded));
        // ~4 ms later (> 1/300 s) a token has refilled, so a client that respected
        // the rate is admitted again rather than dropped.
        let t1 = t0 + Duration::from_millis(4);
        assert_eq!(budget.admit(t1), Ok(()));
    }
    #[test]
    fn established_backlog_spends_only_real_tokens_and_recovers_without_extra_credit() {
        use super::BudgetAdmission;
        let now = Instant::now();
        let mut budget = ServerboundBudget::new(now, 300.0, 600.0);
        budget.establish_play();
        for _ in 0..600 {
            assert_eq!(
                budget.admit_or_defer(now, false),
                Ok(BudgetAdmission::Admitted)
            );
        }
        let Ok(BudgetAdmission::Deferred(ready)) = budget.admit_or_defer(now, false) else {
            panic!("one retained frame waits for a token");
        };
        assert!(ready > now);
        assert_eq!(budget.budget.available_tokens(), 0.0);
        assert_eq!(
            budget.admit_or_defer(ready, true),
            Ok(BudgetAdmission::Admitted)
        );
        assert!(budget.budget.available_tokens() < 0.00001);
        let refilled = ready + Duration::from_secs(1);
        assert_eq!(
            budget.admit_or_defer(refilled, false),
            Ok(BudgetAdmission::Admitted)
        );
        assert_eq!(
            budget.recovery_until, None,
            "half-burst refill ends recovery"
        );
        assert_eq!(budget.over_budget(), 0);
    }

    #[test]
    fn login_pipeline_remains_strict_and_continuous_recovery_has_a_hard_limit() {
        use super::BudgetAdmission;
        let now = Instant::now();
        let mut strict = ServerboundBudget::new(now, 300.0, 600.0);
        for _ in 0..600 {
            assert_eq!(
                strict.admit_or_defer(now, false),
                Ok(BudgetAdmission::Admitted)
            );
        }
        assert_eq!(
            strict.admit_or_defer(now, false),
            Err(DisconnectReason::BudgetExceeded)
        );
        let mut paced = ServerboundBudget::new(now, 300.0, 600.0);
        paced.establish_play();
        for _ in 0..600 {
            paced.admit_or_defer(now, false).expect("burst");
        }
        let mut at = now;
        loop {
            match paced.admit_or_defer(at, false) {
                Ok(BudgetAdmission::Admitted) => {}
                Ok(BudgetAdmission::Deferred(ready)) => at = ready,
                Err(reason) => {
                    assert_eq!(reason, DisconnectReason::BudgetExceeded);
                    break;
                }
            }
        }
        assert_eq!(at, now + Duration::from_secs(2));
        assert_eq!(paced.over_budget(), 1);
    }

    #[test]
    fn invalid_quota_fails_closed_and_recovery_never_exceeds_two_seconds() {
        use super::BudgetAdmission;
        let now = Instant::now();
        for rate in [0.0, f64::NAN, f64::INFINITY] {
            let mut budget = ServerboundBudget::new(now, rate, 0.0);
            budget.establish_play();
            assert_eq!(
                budget.admit_or_defer(now, false),
                Err(DisconnectReason::BudgetExceeded)
            );
        }
        let mut budget = ServerboundBudget::new(now, 1.0, 10.0);
        budget.establish_play();
        for _ in 0..10 {
            budget.admit_or_defer(now, false).expect("burst");
        }
        assert!(matches!(
            budget.admit_or_defer(now, false),
            Ok(BudgetAdmission::Deferred(_))
        ));
        assert_eq!(budget.recovery_until, Some(now + Duration::from_secs(2)));
    }
    #[test]
    fn quota_retry_cannot_reset_a_small_burst_recovery_episode() {
        use super::BudgetAdmission;
        let now = Instant::now();
        for (rate, burst) in [(1.0, 1.0), (2.0, 2.0)] {
            let mut budget = ServerboundBudget::new(now, rate, burst);
            budget.establish_play();
            for _ in 0..burst as usize {
                budget.admit_or_defer(now, false).expect("burst");
            }
            let mut at = now;
            let mut retry = false;
            loop {
                match budget.admit_or_defer(at, retry) {
                    Ok(BudgetAdmission::Admitted) => retry = false,
                    Ok(BudgetAdmission::Deferred(ready)) => {
                        at = ready;
                        retry = true;
                    }
                    Err(reason) => {
                        assert_eq!(reason, DisconnectReason::BudgetExceeded);
                        break;
                    }
                }
            }
            assert_eq!(at, now + Duration::from_secs(1));
        }
    }
}
