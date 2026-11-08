# Monitoring answer for the application

Prepared 9 November 2026. This is planned funded work; no continuously running worker or alert service is claimed. The existing manual smoke-test feed remains disclosed.

## Text to use in the form

CoverFi will monitor the protection engine, reserve vault, custody vault and oracle adapter for its single stablecoin market. The dashboard will show the exact network/contract IDs, oracle observation age, available claim capacity, locked liabilities, reserved claims and accounting reconciliation. We will detect stale or unavailable prices, positions awaiting an expiry observation, failed/repeated claim attempts, unexpected role/configuration changes and reserve/token-balance mismatches. Each alert will have an assigned operator, documented response and a test demonstrating the notification path. Price simulations and mock assets will remain visibly labeled on testnet.

The current source enforces quote freshness and expiry-observation eligibility, and refuses new positions when capacity/risk checks fail. Monitoring will not be described as an automatic pause mechanism unless the actual implementation supports and tests that action. Claims, settlement and principal exit are checked separately from the sales pause. We will publish the tranche 2 threat-model/monitoring evidence and a redacted operating summary; detailed response channels and operator secrets remain internal.

Current limitation: testnet publishers are team-controlled, the deployed source has no external upstream enabled, and no independent publisher operation or continuous production feed has been established. Mainnet activation requires a verified market-pair feed, appropriate governance and reviewed operator readiness.
