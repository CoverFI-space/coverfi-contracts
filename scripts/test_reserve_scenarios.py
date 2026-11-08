import unittest
from reserve_scenarios import UNIT, SCALE, scenario, report


class ReserveStressTests(unittest.TestCase):
    def test_each_scenario_conserves_provider_and_restricted_assets(self):
        for row in report()["scenarios"]:
            self.assertEqual(row["totalAssetsAfterSettlementAndClaim"],
                             row["providerNavAfterSettlementAndClaim"] + row["safety"] + row["automationAfterKeeper"])
            self.assertEqual(row["riskPremium"] + row["automationFee"],
                             row["underwriting"] + row["protocol"] + row["safety"] + row["automationAfterKeeper"] + row["keeperReward"])

    def test_deeper_losses_never_exceed_locked_cap(self):
        rows = report()["scenarios"]
        self.assertEqual(rows[0]["payout"], 0)
        self.assertEqual(rows[2]["payout"], 300 * UNIT)
        self.assertEqual(rows[-1]["payout"], 500 * UNIT)
        self.assertEqual(rows[-1]["remainingUncompensatedPriceLoss"], 1500 * UNIT)
        self.assertEqual([r["payout"] for r in rows], sorted(r["payout"] for r in rows))
        self.assertTrue(all(r["payout"] <= r["maximumPayout"] for r in rows))

    def test_uncollateralizable_scenario_is_rejected(self):
        with self.assertRaises(ValueError):
            scenario(100 * UNIT, 5000 * UNIT, SCALE, 94_000_000)

    def test_zero_price_is_not_mislabeled_as_supported_settlement(self):
        with self.assertRaises(ValueError):
            scenario(1000 * UNIT, 5000 * UNIT, SCALE, 0)
