"""Deterministic reserve stress arithmetic, in test payout-token base units.

Not an actuarial price, loss probability estimate, investment forecast or audit.
The CA must review assumptions and reconcile the selected deployed release.
"""
import argparse
import json
from pathlib import Path

UNIT = 10_000_000
BPS = 10_000
SCALE = 100_000_000


def scenario(reserve, protected_amount, entry_price, settlement_price,
             max_payout_bps=1000, premium_bps=170):
    if min(reserve, protected_amount, entry_price, settlement_price) <= 0:
        raise ValueError("Positive values required; zero-price settlement is unsupported")
    if not 0 < max_payout_bps <= 1000 or not 0 < premium_bps <= 1000:
        raise ValueError("Invalid prototype cap or premium")
    notional = protected_amount * entry_price // SCALE
    cap = notional * max_payout_bps // BPS
    # One protected market: its 75% concentration limit is stricter than the
    # 90% utilization limit for this isolated, initially unencumbered pool.
    if cap <= 0 or cap * BPS // reserve > 7500:
        raise ValueError("Exposure exceeds the source concentration bound")
    premium = notional * premium_bps // BPS
    underwriting = premium * 7800 // BPS
    protocol = premium * 1200 // BPS
    safety = premium * 700 // BPS
    automation_fee = 500_000
    keeper = 100_000
    automation = premium - underwriting - protocol - safety + automation_fee
    loss = protected_amount * max(entry_price - settlement_price, 0) // SCALE
    payout = min(loss, cap)
    total_assets = reserve + premium + automation_fee - protocol - keeper - payout
    automation_after = automation - keeper
    provider_nav = reserve + underwriting - payout
    return {
        "settlementPriceScaled": settlement_price, "entryNotional": notional,
        "maximumPayout": cap, "riskPremium": premium, "automationFee": automation_fee,
        "underwriting": underwriting, "protocol": protocol, "safety": safety,
        "automationAfterKeeper": automation_after, "keeperReward": keeper,
        "uncappedPriceLoss": loss, "payout": payout,
        "remainingUncompensatedPriceLoss": loss - payout,
        "providerNavAfterSettlementAndClaim": provider_nav,
        "totalAssetsAfterSettlementAndClaim": total_assets,
    }


def report():
    return {
        "status": "illustrative_arithmetic_not_actuarially_validated",
        "unit": "seven-decimal test payout-token units; not USD or real capital",
        "assumptions": {
            "initialProviderReserve": 1000 * UNIT,
            "protectedAmount": 5000 * UNIT, "entryPriceScaled": SCALE,
            "maximumPayoutBps": 1000, "assumedRiskPremiumBps": 170,
            "note": "One hypothetical seven-day position, zero prior exposure/volatility, "
                    "50% projected utilization/concentration. The 170 bps assumption "
                    "illustrates current base/surcharge arithmetic, not a live quote. "
                    "No starting safety/automation balance; fee receipts fund the modeled keeper. "
                    "This is unrelated to any LP commitment target.",
        },
        "scenarios": [scenario(1000 * UNIT, 5000 * UNIT, SCALE, p)
                      for p in (100_000_000, 98_000_000, 94_000_000, 90_000_000, 60_000_000)],
        "limitations": ["No probability or expected-loss calibration.",
                        "No market demand or willingness-to-pay evidence.",
                        "Prices assume the declared payout-token quote unit.",
                        "A cap leaves losses above that cap uncompensated.",
                        "No oracle outage/archival timing or issuer default is priced.",
                        "All same-asset positions may lose simultaneously.",
                        "No independent or CA sign-off is claimed."],
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    text = json.dumps(report(), indent=2) + "\n"
    if args.output:
        args.output.write_text(text, encoding="utf-8")
    else:
        print(text, end="")
