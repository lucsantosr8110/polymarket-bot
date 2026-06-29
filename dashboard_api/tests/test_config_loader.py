import unittest

from dashboard_api.config_loader import (
    DEFAULT_GLOBAL_CONFIG,
    DEFAULT_STRATEGIES,
    merge_global_config,
    merge_strategy,
)


class ConfigLoaderTests(unittest.TestCase):
    def test_merge_strategy_updates_only_named_strategy(self):
        updated = merge_strategy(DEFAULT_STRATEGIES, "Balanced", {"min_confidence": 0.3})

        balanced = next(item for item in updated if item["name"] == "Balanced")
        aggressive = next(item for item in updated if item["name"] == "Aggressive")

        self.assertEqual(balanced["min_confidence"], 0.3)
        self.assertEqual(balanced["min_edge"], 0.06)
        self.assertEqual(aggressive["min_confidence"], 0.4)

    def test_merge_strategy_is_case_insensitive(self):
        updated = merge_strategy(DEFAULT_STRATEGIES, "balanced", {"min_bet": 6.0})

        balanced = next(item for item in updated if item["name"] == "Balanced")

        self.assertEqual(balanced["min_bet"], 6.0)

    def test_merge_strategy_rejects_unknown_strategy(self):
        with self.assertRaises(KeyError):
            merge_strategy(DEFAULT_STRATEGIES, "Ghost", {"min_bet": 6.0})

    def test_merge_global_config_keeps_existing_defaults(self):
        updated = merge_global_config(DEFAULT_GLOBAL_CONFIG, {"news_enabled": True})

        self.assertTrue(updated["news_enabled"])
        self.assertEqual(updated["scan_interval_mins"], 30)
        self.assertEqual(updated["sidecar_timeout_secs"], 10)


if __name__ == "__main__":
    unittest.main()
