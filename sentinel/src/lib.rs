pub mod approval;
pub mod config;
pub mod guardrails;
pub mod input;
pub mod keychain;
pub mod ledger;
pub mod permissions;

#[cfg(feature = "dev-http")]
pub mod dev_http;

use anyhow::{Context, Result};
use guardrails::{Decision, Guardrails};
use input::{InputAction, InputBackend};
use ledger::Ledger;
use std::sync::Arc;

pub struct Sentinel {
    pub ledger: Arc<Ledger>,
    pub guardrails: Arc<Guardrails>,
    pub input: Arc<dyn InputBackend>,
    pub approvals: Arc<approval::ApprovalClient>,
    pub device_id: String,
    pub cfg: config::Config,
}

impl Sentinel {
    pub fn new(cfg: config::Config) -> Result<Self> {
        let store = ledger::SqliteStore::open(&cfg.ledger_path)?;
        let ledger = Arc::new(Ledger::open(Box::new(store))?);

        #[cfg(feature = "dev-http")]
        let input: Arc<dyn InputBackend> = Arc::new(input::mock::MockInputBackend::default());
        #[cfg(not(feature = "dev-http"))]
        let input: Arc<dyn InputBackend> = Arc::from(input::default_backend());

        Ok(Self {
            ledger,
            guardrails: Arc::new(Guardrails::new(cfg.policy.clone())),
            input,
            approvals: Arc::new(approval::ApprovalClient::new(
                cfg.webhook_url.clone(),
                cfg.device_id.clone(),
            )),
            device_id: cfg.device_id.clone(),
            cfg,
        })
    }

    pub async fn perform(
        &self,
        action_name: &str,
        intent: &str,
        cost_usd: f64,
        action: InputAction,
    ) -> Result<()> {
        match self.guardrails.evaluate(action_name, intent, cost_usd)? {
            Decision::Deny(reason) => {
                let _ = self.ledger.append(
                    "guardrail.deny",
                    serde_json::to_vec(&serde_json::json!({
                        "action": action_name,
                        "intent": intent,
                        "reason": reason
                    }))?
                    .as_slice(),
                );
                anyhow::bail!("denied: {reason}");
            }
            Decision::RequireApproval(reason) => {
                let summary = format!("{action_name}: {intent}");
                let status = self
                    .approvals
                    .request(action_name, &summary, cost_usd)
                    .await?;
                let _ = self.ledger.append(
                    "approval.outcome",
                    serde_json::to_vec(&serde_json::json!({
                        "action": action_name,
                        "intent": intent,
                        "reason": reason,
                        "status": format!("{status:?}")
                    }))?
                    .as_slice(),
                );
                if status != approval::ApprovalStatus::Approved {
                    anyhow::bail!("not approved: {status:?}");
                }
            }
            Decision::Allow => {}
        }

        self.ledger.append(
            "action.begin",
            serde_json::to_vec(&serde_json::json!({
                "action": action_name,
                "intent": intent,
                "cost_usd": cost_usd
            }))?
            .as_slice(),
        )?;

        self.input.execute(&action).context("input injection")?;
        self.guardrails.commit_spend(cost_usd);

        self.ledger.append(
            "action.end",
            serde_json::to_vec(&serde_json::json!({
                "action": action_name,
                "backend": self.input.name()
            }))?
            .as_slice(),
        )?;
        Ok(())
    }

    pub fn verify_all(&self) -> Result<u64> {
        let store = ledger::SqliteStore::open(&self.cfg.ledger_path)?;
        let all = store.all()?;
        ledger::verify_chain(&all)?;
        Ok(all.last().map(|r| r.seq + 1).unwrap_or(0))
    }

    pub fn export_to_desktop(&self) -> Result<std::path::PathBuf> {
        let store = ledger::SqliteStore::open(&self.cfg.ledger_path)?;
        let all = store.all()?;
        ledger::verify_chain(&all)?;
        let dir = dirs::desktop_dir().ok_or_else(|| anyhow::anyhow!("no desktop dir"))?;
        let path = dir.join(format!(
            "sovereign-ledger-{}.json",
            chrono::Utc::now().format("%Y%m%dT%H%M%SZ")
        ));
        std::fs::write(&path, serde_json::to_vec_pretty(&all)?)?;
        Ok(path)
    }
}
