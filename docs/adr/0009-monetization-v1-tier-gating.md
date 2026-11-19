# ADR-0009: v1.0 Monetization — Provider-Agnostic Tier Gating, No Checkout Yet

- Status: accepted
- Date: 2026-11-17
- Leaves open: PRD §14 Q9 (payment provider)

## Context

PRD §9 defines Free and Pro tiers. Real web subscriptions need a provider
account, keys, and a webhook backend for entitlement updates; the provider
choice (Stripe vs a Merchant-of-Record like Lemonsqueezy or Paddle) is still
open. The owner chose to build the gating without checkout.

## Decision

The web shell implements an `EntitlementProvider` interface and the PRD §9.1
limits: 50 imports/month, 15 AI prompts/month, basic sliders only, histogram
only, 2K export with a watermark, 5 Recipes, and no LUT import/export or A/B on
Free. Usage counters live locally (ADR-0010). A `BETA_ALL_PRO` flag grants Pro
to everyone during the beta, so no user is blocked by an unpurchasable tier.

## Consequences

- Wiring a provider later means implementing one interface plus a webhook
  endpoint, not reworking the gating logic.
- Local counters can be reset by clearing site data. That's acceptable for a
  beta, and it's addressed when entitlements become server-verified.
- Q9 must be decided (new ADR) before `BETA_ALL_PRO` is turned off.
