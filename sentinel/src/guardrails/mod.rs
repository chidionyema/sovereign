use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailPolicy {
    pub allowed_actions: Vec<String>,
    pub denied_actions: Vec<String>,
    pub max_actions_per_minute: u32,
    pub daily_budget_usd: f64,
    pub require_approval_above_usd: f64,
    pub semantic_deny: Vec<String>,
}

impl Default for GuardrailPolicy {
    fn default() -> Self {
        Self {
            allowed_actions: vec![],
            denied_actions: vec![],
            max_actions_per_minute: 60,
            daily_budget_usd: 25.0,
            require_approval_above_usd: 5.0,
            semantic_deny: vec![
                "wire transfer".into(),
                "delete account".into(),
                "format c:".into(),
                "rm -rf".into(),
            ],
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Allow,
    RequireApproval(String),
    Deny(String),
}

pub struct Guardrails {
    policy: GuardrailPolicy,
    window: parking_lot::Mutex<Vec<Instant>>,
    spend_today: parking_lot::Mutex<f64>,
}

impl Guardrails {
    pub fn new(policy: GuardrailPolicy) -> Self {
        Self {
            policy,
            window: parking_lot::Mutex::new(Vec::new()),
            spend_today: parking_lot::Mutex::new(0.0),
        }
    }

    pub fn evaluate(&self, action: &str, intent: &str, cost_usd: f64) -> Result<Decision> {
        if self.policy.denied_actions.iter().any(|d| d == action) {
            return Ok(Decision::Deny(format!("action '{action}' explicitly denied")));
        }
        if !self.policy.allowed_actions.is_empty()
            && !self.policy.allowed_actions.iter().any(|a| a == action)
        {
            return Ok(Decision::Deny(format!("action '{action}' not in allowlist")));
        }

        let lower = intent.to_lowercase();
        if let Some(hit) = self
            .policy
            .semantic_deny
            .iter()
            .find(|b| lower.contains(&b.to_lowercase()))
        {
            return Ok(Decision::Deny(format!("semantic boundary: '{hit}'")));
        }

        {
            let mut w = self.window.lock();
            let cutoff = Instant::now() - Duration::from_secs(60);
            w.retain(|t| *t > cutoff);
            if w.len() as u32 >= self.policy.max_actions_per_minute {
                return Ok(Decision::Deny("rate limit exceeded".into()));
            }
            w.push(Instant::now());
        }

        let spent = *self.spend_today.lock();
        if spent + cost_usd > self.policy.daily_budget_usd {
            return Ok(Decision::Deny(format!(
                "daily budget exceeded: {:.2} + {:.2} > {:.2}",
                spent, cost_usd, self.policy.daily_budget_usd
            )));
        }
        if cost_usd >= self.policy.require_approval_above_usd {
            return Ok(Decision::RequireApproval(format!(
                "cost ${:.2} ≥ threshold ${:.2}",
                cost_usd, self.policy.require_approval_above_usd
            )));
        }
        Ok(Decision::Allow)
    }

    pub fn commit_spend(&self, cost_usd: f64) {
        *self.spend_today.lock() += cost_usd;
    }
}
