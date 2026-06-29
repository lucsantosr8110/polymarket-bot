-- ADR 009: Persist the effective fee rate used for a bet when available.
-- Historical rows can derive an approximate rate from fee_paid / cost.
ALTER TABLE bets ADD COLUMN IF NOT EXISTS fee_rate DOUBLE PRECISION;

UPDATE bets
SET fee_rate = fee_paid / NULLIF(cost, 0)
WHERE fee_rate IS NULL
  AND fee_paid IS NOT NULL
  AND cost IS NOT NULL
  AND cost <> 0;
