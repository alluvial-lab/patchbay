use std::{
    collections::HashMap,
    fmt::Write as _,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginLimitConfig {
    pub window: Duration,
    pub account_max_failures: u32,
    pub network_max_failures: u32,
    pub max_concurrent_verifications: u32,
    pub max_tracked_accounts: usize,
    pub max_tracked_networks: usize,
}

impl Default for LoginLimitConfig {
    fn default() -> Self {
        Self {
            window: Duration::from_secs(60),
            account_max_failures: 5,
            network_max_failures: 5,
            max_concurrent_verifications: 2,
            max_tracked_accounts: 1_024,
            max_tracked_networks: 1_024,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginLimitDimension {
    Account,
    Network,
}

impl LoginLimitDimension {
    fn as_str(self) -> &'static str {
        match self {
            Self::Account => "account",
            Self::Network => "network",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginLimitExceeded {
    pub blocked_dimensions: Vec<LoginLimitDimension>,
    pub retry_after: Duration,
}

#[derive(Debug, Clone)]
struct AttemptWindow {
    started_at: Instant,
    failures: u32,
    in_flight: u32,
    last_touched_at: Instant,
}

impl AttemptWindow {
    fn new(now: Instant) -> Self {
        Self {
            started_at: now,
            failures: 0,
            in_flight: 0,
            last_touched_at: now,
        }
    }

    fn refresh(&mut self, now: Instant, window: Duration) {
        if now.duration_since(self.started_at) >= window {
            self.started_at = now;
            self.failures = 0;
        }
        self.last_touched_at = now;
    }
}

#[derive(Debug, Default)]
struct LoginLimitState {
    accounts: HashMap<String, AttemptWindow>,
    networks: HashMap<String, AttemptWindow>,
}

#[derive(Clone)]
pub struct LoginLimiter {
    config: LoginLimitConfig,
    state: Arc<Mutex<LoginLimitState>>,
    now: Arc<dyn Fn() -> Instant + Send + Sync>,
}

impl std::fmt::Debug for LoginLimiter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LoginLimiter")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl LoginLimiter {
    pub fn new(config: LoginLimitConfig) -> Result<Self, String> {
        Self::new_with_clock(config, Instant::now)
    }

    pub fn new_with_clock<F>(config: LoginLimitConfig, now: F) -> Result<Self, String>
    where
        F: Fn() -> Instant + Send + Sync + 'static,
    {
        if config.window.is_zero()
            || config.account_max_failures == 0
            || config.network_max_failures == 0
            || config.max_concurrent_verifications == 0
            || config.max_tracked_accounts == 0
            || config.max_tracked_networks == 0
        {
            return Err("login limiter values must be positive".to_owned());
        }
        Ok(Self {
            config,
            state: Arc::new(Mutex::new(LoginLimitState::default())),
            now: Arc::new(now),
        })
    }

    pub fn begin_attempt(
        &self,
        actor_id: &str,
        network_address: &str,
    ) -> Result<LoginAttempt, LoginLimitExceeded> {
        let now = (self.now)();
        let mut state = self.state.lock().expect("login limiter mutex poisoned");
        prune_windows(&mut state.accounts, now, self.config.window);
        prune_windows(&mut state.networks, now, self.config.window);
        if !state.accounts.contains_key(actor_id)
            && state.accounts.len() >= self.config.max_tracked_accounts
        {
            evict_oldest_idle_window(&mut state.accounts);
            if state.accounts.len() >= self.config.max_tracked_accounts {
                return Err(LoginLimitExceeded {
                    blocked_dimensions: vec![LoginLimitDimension::Account],
                    retry_after: Duration::from_secs(1),
                });
            }
        }
        if !state.networks.contains_key(network_address)
            && state.networks.len() >= self.config.max_tracked_networks
        {
            evict_oldest_idle_window(&mut state.networks);
            if state.networks.len() >= self.config.max_tracked_networks {
                return Err(LoginLimitExceeded {
                    blocked_dimensions: vec![LoginLimitDimension::Network],
                    retry_after: Duration::from_secs(1),
                });
            }
        }

        let account = state
            .accounts
            .entry(actor_id.to_owned())
            .or_insert_with(|| AttemptWindow::new(now));
        account.refresh(now, self.config.window);
        let account_blocked = account.failures >= self.config.account_max_failures
            || account.in_flight >= self.config.max_concurrent_verifications;
        let account_retry = retry_after(account, now, self.config.window);

        let network = state
            .networks
            .entry(network_address.to_owned())
            .or_insert_with(|| AttemptWindow::new(now));
        network.refresh(now, self.config.window);
        let network_blocked = network.failures >= self.config.network_max_failures
            || network.in_flight >= self.config.max_concurrent_verifications;
        let network_retry = retry_after(network, now, self.config.window);

        let mut blocked_dimensions = Vec::new();
        let mut retry = Duration::ZERO;
        if account_blocked {
            blocked_dimensions.push(LoginLimitDimension::Account);
            retry = retry.max(account_retry);
        }
        if network_blocked {
            blocked_dimensions.push(LoginLimitDimension::Network);
            retry = retry.max(network_retry);
        }
        if !blocked_dimensions.is_empty() {
            return Err(LoginLimitExceeded {
                blocked_dimensions,
                retry_after: retry.max(Duration::from_millis(1)),
            });
        }

        state
            .accounts
            .get_mut(actor_id)
            .expect("account window was inserted")
            .in_flight += 1;
        state
            .networks
            .get_mut(network_address)
            .expect("network window was inserted")
            .in_flight += 1;
        Ok(LoginAttempt {
            limiter: self.clone(),
            actor_id: actor_id.to_owned(),
            network_address: network_address.to_owned(),
            completed: false,
        })
    }

    fn complete(&self, actor_id: &str, network_address: &str, success: bool) {
        let now = (self.now)();
        let mut state = self.state.lock().expect("login limiter mutex poisoned");
        if let Some(account) = state.accounts.get_mut(actor_id) {
            complete_window(
                account,
                now,
                self.config.window,
                self.config.account_max_failures,
                success,
            );
        }
        if let Some(network) = state.networks.get_mut(network_address) {
            complete_window(
                network,
                now,
                self.config.window,
                self.config.network_max_failures,
                success,
            );
        }
    }

    fn release(&self, actor_id: &str, network_address: &str) {
        let mut state = self.state.lock().expect("login limiter mutex poisoned");
        if let Some(account) = state.accounts.get_mut(actor_id) {
            account.in_flight = account.in_flight.saturating_sub(1);
        }
        if let Some(network) = state.networks.get_mut(network_address) {
            network.in_flight = network.in_flight.saturating_sub(1);
        }
    }
}

impl Default for LoginLimiter {
    fn default() -> Self {
        Self::new(LoginLimitConfig::default()).expect("default login limiter config is valid")
    }
}

#[derive(Debug)]
pub struct LoginAttempt {
    limiter: LoginLimiter,
    actor_id: String,
    network_address: String,
    completed: bool,
}

impl LoginAttempt {
    pub fn success(mut self) {
        self.limiter
            .complete(&self.actor_id, &self.network_address, true);
        self.completed = true;
    }

    pub fn failure(mut self) {
        self.limiter
            .complete(&self.actor_id, &self.network_address, false);
        self.completed = true;
    }
}

impl Drop for LoginAttempt {
    fn drop(&mut self) {
        if !self.completed {
            self.limiter.release(&self.actor_id, &self.network_address);
        }
    }
}

fn complete_window(
    window: &mut AttemptWindow,
    now: Instant,
    duration: Duration,
    max_failures: u32,
    success: bool,
) {
    window.refresh(now, duration);
    window.in_flight = window.in_flight.saturating_sub(1);
    if success {
        window.failures = 0;
        window.started_at = now;
    } else {
        window.failures = window.failures.saturating_add(1).min(max_failures);
    }
}

fn retry_after(window: &AttemptWindow, now: Instant, duration: Duration) -> Duration {
    if window.failures > 0 {
        duration.saturating_sub(now.duration_since(window.started_at))
    } else {
        Duration::from_secs(1)
    }
}

fn prune_windows(windows: &mut HashMap<String, AttemptWindow>, now: Instant, window: Duration) {
    windows.retain(|_, attempt| {
        attempt.in_flight > 0 || now.duration_since(attempt.started_at) < window
    });
}

fn evict_oldest_idle_window(windows: &mut HashMap<String, AttemptWindow>) {
    let oldest = windows
        .iter()
        .filter(|(_, window)| window.in_flight == 0)
        .min_by_key(|(_, window)| window.last_touched_at)
        .map(|(address, _)| address.clone());
    if let Some(address) = oldest {
        windows.remove(&address);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginAuditOutcome {
    Success,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginAuditEvent {
    pub operator_actor_id: String,
    pub direct_socket_address: String,
    pub outcome: LoginAuditOutcome,
    pub reason: &'static str,
    pub blocked_dimensions: Vec<LoginLimitDimension>,
}

impl LoginAuditEvent {
    #[must_use]
    pub fn redacted_line(&self) -> String {
        let mut line = format!(
            "audit_event=interactive_login outcome={} reason={} operator_actor_id={} direct_socket_address={}",
            match self.outcome {
                LoginAuditOutcome::Success => "success",
                LoginAuditOutcome::Failure => "failure",
            },
            self.reason,
            safe_log_value(&self.operator_actor_id),
            safe_log_value(&self.direct_socket_address),
        );
        if !self.blocked_dimensions.is_empty() {
            let dimensions = self
                .blocked_dimensions
                .iter()
                .map(|dimension| dimension.as_str())
                .collect::<Vec<_>>()
                .join(",");
            let _ = write!(line, " blocked_dimensions={dimensions}");
        }
        line
    }
}

pub trait LoginAuditSink: Send + Sync {
    fn record(&self, event: LoginAuditEvent);
}

#[derive(Debug, Default)]
pub struct StderrLoginAuditSink;

impl LoginAuditSink for StderrLoginAuditSink {
    fn record(&self, event: LoginAuditEvent) {
        eprintln!("{}", event.redacted_line());
    }
}

fn safe_log_value(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_graphic() && character != '=' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_verification_cap_is_shared_by_account_and_network() {
        let limiter = LoginLimiter::new(LoginLimitConfig {
            max_concurrent_verifications: 1,
            ..LoginLimitConfig::default()
        })
        .unwrap();
        let attempt = limiter.begin_attempt("operator", "127.0.0.1").unwrap();
        let blocked = limiter
            .begin_attempt("operator", "127.0.0.2")
            .expect_err("account concurrency must cap parallel scrypt calls");
        assert_eq!(
            blocked.blocked_dimensions,
            vec![LoginLimitDimension::Account]
        );
        drop(attempt);
        assert!(limiter.begin_attempt("operator", "127.0.0.2").is_ok());
    }

    #[test]
    fn audit_line_is_structured_and_contains_no_secret_fields() {
        let event = LoginAuditEvent {
            operator_actor_id: "operator\nforged=field".to_owned(),
            direct_socket_address: "127.0.0.1".to_owned(),
            outcome: LoginAuditOutcome::Failure,
            reason: "invalid_credentials",
            blocked_dimensions: Vec::new(),
        };
        let line = event.redacted_line();
        assert!(line.contains("operator_actor_id=operator_forged_field"));
        assert!(line.contains("direct_socket_address=127.0.0.1"));
        assert!(!line.contains("password"));
        assert!(!line.contains("scrypt"));
        assert!(!line.contains('\n'));
    }
}
