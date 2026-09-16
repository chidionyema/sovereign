from __future__ import annotations

import asyncio
import hashlib
import hmac
import os
import secrets
import time
from typing import Literal

import httpx
import structlog
from fastapi import FastAPI, HTTPException, Request
from fastapi.responses import HTMLResponse
from pydantic import BaseModel, Field

log = structlog.get_logger()


class ApprovalStore:
    """In-memory approval tracking. Swap for Redis in production."""

    def __init__(self) -> None:
        self._d: dict[str, dict] = {}

    def put_if_absent(self, nonce: str, value: dict) -> bool:
        if nonce in self._d:
            return False
        self._d[nonce] = value
        return True

    def get(self, nonce: str) -> dict | None:
        return self._d.get(nonce)

    def set_decision(self, nonce: str, approve: bool, actor: str) -> bool:
        v = self._d.get(nonce)
        if v is None or v["decision"] is not None:
            return False
        v["decision"] = approve
        v["actor"] = actor
        v["decided_at"] = time.time()
        return True


STORE = ApprovalStore()
SERVER_SECRET = os.environ.get("OOB_SECRET", secrets.token_hex(32)).encode("utf-8")
TOKEN_TTL_S = 300
LONG_POLL_S = 60


def _sign(nonce: str, decision: str, exp: int) -> str:
    msg = f"{nonce}:{decision}:{exp}".encode("utf-8")
    return hmac.new(SERVER_SECRET, msg, hashlib.sha256).hexdigest()


def _verify(nonce: str, decision: str, exp: int, sig: str) -> bool:
    if exp < int(time.time()):
        return False
    return hmac.compare_digest(_sign(nonce, decision, exp), sig)


class TwilioClient:
    def __init__(self, account_sid: str, auth_token: str, from_number: str) -> None:
        self.sid = account_sid
        self.token = auth_token
        self.from_ = from_number
        self.base = f"https://api.twilio.com/2010-04-01/Accounts/{account_sid}"
        self.http = httpx.AsyncClient(timeout=10.0, auth=(account_sid, auth_token))

    async def send_sms(self, to: str, body: str) -> None:
        r = await self.http.post(
            f"{self.base}/Messages.json",
            data={"From": self.from_, "To": to, "Body": body},
        )
        r.raise_for_status()

    async def place_call(self, to: str, twiml_url: str) -> None:
        r = await self.http.post(
            f"{self.base}/Calls.json",
            data={"From": self.from_, "To": to, "Url": twiml_url, "Timeout": 20},
        )
        r.raise_for_status()


class OOBRequest(BaseModel):
    device_id: str = Field(min_length=1, max_length=128)
    action: str = Field(min_length=1, max_length=256)
    summary: str = Field(min_length=1, max_length=512)
    cost_usd: float = Field(ge=0.0, le=10_000.0)
    nonce: str = Field(min_length=8, max_length=128)


class OOBResponse(BaseModel):
    status: Literal["approved", "denied", "pending"]
    reason: str | None = None


def build_app(
    *,
    public_url: str,
    twilio: TwilioClient,
    phone_to: str,
    voice_fallback_after_s: int = 25,
) -> FastAPI:
    app = FastAPI()

    @app.post("/request", response_model=OOBResponse)
    async def oob_request(req: OOBRequest) -> OOBResponse:
        if not STORE.put_if_absent(
            req.nonce,
            {
                "device_id": req.device_id,
                "action": req.action,
                "summary": req.summary,
                "cost_usd": req.cost_usd,
                "decision": None,
                "created_at": time.time(),
            },
        ):
            raise HTTPException(409, "nonce reused")

        exp = int(time.time()) + TOKEN_TTL_S
        approve_url = _decide_url(public_url, req.nonce, "approve", exp)
        deny_url = _decide_url(public_url, req.nonce, "deny", exp)

        body = (
            f"Sovereign wants to: {req.summary}\n"
            f"Action: {req.action}  •  Cost: ${req.cost_usd:.2f}\n\n"
            f"APPROVE: {approve_url}\n"
            f"DENY:    {deny_url}\n\n"
            f"(expires in {TOKEN_TTL_S // 60} min)"
        )

        sms_task = asyncio.create_task(_safe(twilio.send_sms(phone_to, body), "sms"))
        voice_task = asyncio.create_task(
            _voice_after(
                twilio,
                req.nonce,
                req.summary,
                req.action,
                req.cost_usd,
                phone_to,
                voice_fallback_after_s,
            )
        )
        decision = await _wait_for_decision(req.nonce, LONG_POLL_S)

        for t in (sms_task, voice_task):
            if not t.done():
                t.cancel()

        if decision is None:
            log.warning("oob.timeout", nonce=req.nonce, action=req.action)
            return OOBResponse(status="pending", reason="timeout")

        approved, actor = decision
        log.info("oob.decided", nonce=req.nonce, approved=approved, actor=actor)
        return OOBResponse(
            status="approved" if approved else "denied", reason=f"actor={actor}"
        )

    @app.get("/decide", response_class=HTMLResponse)
    async def decide(n: str, d: str, e: int, s: str) -> HTMLResponse:
        if d not in ("approve", "deny"):
            raise HTTPException(400, "bad decision")
        if not _verify(n, d, e, s):
            return HTMLResponse("<h1>Expired or invalid link.</h1>", status_code=403)
        ok = STORE.set_decision(n, d == "approve", actor="phone.web")
        if not ok:
            return HTMLResponse("<h1>Already decided.</h1>", status_code=409)
        return HTMLResponse(
            f"<h1>{'✅ Approved' if d == 'approve' else '🛑 Denied'}</h1>"
            "<p>You can close this tab.</p>"
        )

    @app.post("/twilio/voice")
    async def twilio_voice(request: Request) -> HTMLResponse:
        form = await request.form()
        nonce = str(form.get("nonce", ""))
        digits = form.get("Digits")
        if digits is None:
            twiml = (
                '<?xml version="1.0" encoding="UTF-8"?>'
                "<Response>"
                '<Gather numDigits="1" timeout="10" action="/oob/twilio/voice">'
                "<Say>Sovereign requests approval. Press 1 to approve. Press 2 to deny.</Say>"
                "</Gather>"
                "<Say>No input received. Denying.</Say>"
                "<Hangup/>"
                "</Response>"
            )
            return HTMLResponse(twiml, media_type="application/xml")

        if digits == "1":
            STORE.set_decision(nonce, True, actor="phone.voice")
        elif digits == "2":
            STORE.set_decision(nonce, False, actor="phone.voice")
        return HTMLResponse(
            '<?xml version="1.0" encoding="UTF-8"?>'
            "<Response><Say>Thank you.</Say><Hangup/></Response>",
            media_type="application/xml",
        )

    return app


def _decide_url(base: str, nonce: str, decision: str, exp: int) -> str:
    sig = _sign(nonce, decision, exp)
    return f"{base}/oob/decide?n={nonce}&d={decision}&e={exp}&s={sig}"


async def _safe(coro, label: str) -> None:
    try:
        await coro
    except Exception as e:
        log.error("oob.side_effect_failed", label=label, err=str(e))


async def _voice_after(
    twilio: TwilioClient,
    nonce: str,
    summary: str,
    action: str,
    cost_usd: float,
    phone_to: str,
    delay_s: int,
) -> None:
    await asyncio.sleep(delay_s)
    if (STORE.get(nonce) or {}).get("decision") is not None:
        return
    twiml_url = f"{os.environ['PUBLIC_URL']}/oob/twilio/voice?nonce={nonce}"
    await twilio.place_call(phone_to, twiml_url)


async def _wait_for_decision(nonce: str, timeout_s: int) -> tuple[bool, str] | None:
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        v = STORE.get(nonce)
        if v is None:
            return None
        if v["decision"] is not None:
            return bool(v["decision"]), str(v.get("actor", "?"))
        await asyncio.sleep(0.5)
    return None
