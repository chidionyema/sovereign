# Sovereign Personal Agent

Production-grade scaffold for a personally-owned, cross-platform (macOS/Windows) GUI automation agent with cryptographic auditing, budget gates, and out-of-band phone approval.

## Architecture

```
┌─────────────────────────────────────────┐
│  Rust Sentinel (Tauri, local)           │
│  • Credential vault (Keychain/CredMgr)  │
│  • Intent guardrails (budget, semantic)  │
│  • Hash-chained ledger (audit trail)     │
│  • OS input injection (Quartz/SendInput) │
└────────────┬────────────────────────────┘
             │ (gRPC / local socket)
┌────────────▼────────────────────────────┐
│  Python Vision Brain (FastAPI)          │
│  • Screenshot → intent + coords          │
│  • Pluggable backends (Anthropic/local)  │
└────────────┬────────────────────────────┘
             │ (HTTPS)
┌────────────▼────────────────────────────┐
│  Concierge Webhook (existing FastAPI)   │
│  • Twilio SMS fan-out (approval)        │
│  • Voice fallback (DTMF)                │
│  • Phone web link (signed)              │
└─────────────────────────────────────────┘
```

## What's Built

### sentinel/ (Rust)
- ✅ **Keychain abstraction** — cross-platform secret store (macOS Keychain, Windows Credential Manager)
- ✅ **Hash-chained ledger** — SQLite with WAL, durable append-only, verification on export
- ✅ **Intent guardrails** — semantic boundaries, budget gates, rate limiting
- ✅ **OS input injection** — macOS Quartz + Windows SendInput with full key-combo support
- ✅ **OOB approval client** — long-poll interface to concierge webhook
- ✅ **Tauri tray UI** — permission status (Accessibility/ScreenRecording), ledger verification/export
- ✅ **Key combo parser** — "cmd+shift+t", "ctrl+alt+del", "f5" → native keycodes

### concierge/oob.py (Python)
- ✅ **FastAPI /oob/request** — receives from Sentinel, returns decision on same connection
- ✅ **Twilio SMS** — fan-out with signed approval links (300s TTL)
- ✅ **Voice fallback** — places call 25s later if phone hasn't acted; DTMF 1=approve, 2=deny
- ✅ **Nonce replay protection** — HMAC-SHA256 signed with server secret
- ✅ **In-memory store** — swap for Redis/Postgres in production

### vision_brain/ (Python)
- ✅ **Stateless planner** — backends for Anthropic Claude (vision) + local VLM (stub)
- ✅ **Anthropic backend** — structured output via tool use, no regex parsing
- ✅ **Base64 validation** — guards against DoS before sending to model

### integration/
- ✅ **End-to-end tests** — Sentinel + Vision Brain + Concierge approval roundtrip
- ✅ **Dev HTTP shim** — test-only REST API over sentinel dev-http feature

## Building & Running

### Sentinel (Rust)

```bash
cd sentinel
cargo build --release
cargo test
```

Dev mode with HTTP shim (for integration tests):
```bash
cargo run --features dev-http -- --dev-http 127.0.0.1:7777
```

### Vision Brain (Python)

```bash
cd vision_brain
pip install -e .[dev]
pytest tests/
python -m uvicorn vision_brain.api:app --reload
```

### Concierge OOB (Python)

```bash
from concierge.oob import build_app, TwilioClient

twilio = TwilioClient(
    account_sid=os.environ["TWILIO_ACCOUNT_SID"],
    auth_token=os.environ["TWILIO_AUTH_TOKEN"],
    from_number=os.environ["TWILIO_FROM"],
)

oob_app = build_app(
    public_url=os.environ["PUBLIC_URL"],
    twilio=twilio,
    phone_to="+15551234567",
)

# Mount onto existing FastAPI app
app.mount("/oob", oob_app)
```

### Integration Tests

```bash
cd integration
pip install -e .[dev]
pytest -m integration
```

## Key Design Decisions

1. **Ledger is load-bearing** — every action, approval, denial, and guardrail hit is written before execution. Crash → you have a receipt showing what failed.

2. **Secrets never touch the LLM** — Vision Brain sees surrogates ("user_credit_card"); Sentinel injects the real value at OS-input time. Even if the model is compromised, no raw secrets in its context.

3. **Modular backends** — OS input (Quartz/SendInput), Vision (Anthropic/local), Approval (Twilio/mock) are all swappable via trait/protocol.

4. **No platform-specific code above input/** — guardrails, ledger, approval are platform-agnostic. OS APIs are isolated.

5. **Fail-closed on permission** — missing Accessibility or ScreenRecording shows a tray warning and prevents action injection.

## What's Open (and Why Ship This Way)

- **Process isolation for Vision Brain** — Add a subprocess/gRPC sidecar for local VLM later; Planner interface is ready.
- **Ledger rollover** — SQLite handles ~10M receipts fine. Switch to per-month files with index table later.
- **Non-English compose** — Windows KEYEVENTF_UNICODE and macOS CGEvent::set_string both handle UTF-8 correctly. Dead-key composition (ñ) requires ToUnicodeEx/TSMInputSource — not worth until a user needs it.
- **Replay window tuning** — 300s is a guess. Profile on your phone; tighten if fast, loosen if slow. Keep it signed & time-bounded.

## Testing

All three layers have tests:

- **Ledger chain verification** — append-only, tamper-detection
- **Guardrails fuzz** — rate limit, budget, semantic boundary edge cases
- **OOB roundtrip** — approval + denial paths with Twilio mock
- **Vision backend fallback** — Anthropic fails → local backend retry
- **End-to-end** — Vision plan → Sentinel guardrails → Concierge approval → ledger

Run all:
```bash
cargo test --workspace && pytest vision_brain/tests concierge/tests integration/ -q
```

## Deployment

### macOS
- Code-sign with Developer ID
- Hardened runtime entitlements (minimal — TCC is external)
- First run: ask Accessibility + ScreenRecording (user approves in System Preferences)
- Ledger exports to ~/Desktop

### Windows
- Build MSIX with visual studio toolchain or cargo-wix
- UAC optional (SendInput works for non-elevated windows)
- Ledger exports to C:\Users\*\Desktop

### CI/CD
- GitHub Actions: Rust on macOS + Windows, Python on Ubuntu, integration on Ubuntu
- Every merge to main: clippy, tests, integration suite
- Release: build native installers, sign, upload to S3

## Config

~/.config/sovereign/config.toml:

```toml
device_id = "macbook-pro"
webhook_url = "https://concierge.example.com"
ledger_path = "/Users/nunn/.local/share/sovereign/ledger.db"

[policy]
allowed_actions = ["browser.click", "browser.type", "search.google"]
denied_actions = []
max_actions_per_minute = 60
daily_budget_usd = 25.0
require_approval_above_usd = 5.0
semantic_deny = ["wire transfer", "delete account"]
```

## License

Private — Nunn only.
