import unittest
from unittest.mock import patch
from contextlib import redirect_stdout
import io
import json
from pathlib import Path
from tempfile import TemporaryDirectory
from deploy_testnet import validate, main


class TestnetBoundary(unittest.TestCase):
    def config(self):
        return dict(network="testnet", admin="G"+"A"*55, treasury="G"+"B"*55,
                    publisher1="G"+"C"*55, publisher2="G"+"D"*55, publisher3="G"+"E"*55,
                    protected_asset="C"+"A"*55, payout_asset="C"+"B"*55,
                    oracle_source="C"+"C"*55, source_account="pilot", source_asset="USDC", source_decimals=8)

    def test_public_testnet_configuration(self):
        validate(self.config())

    def test_non_testnet_rejected(self):
        for network in ("mainnet", "pubnet", "futurenet", "custom"):
            with self.subTest(network=network), self.assertRaises(ValueError):
                validate(dict(self.config(), network=network))

    def test_endpoint_override_rejected(self):
        with self.assertRaises(ValueError):
            validate(dict(self.config(), rpc_url="https://mainnet.sorobanrpc.com"))

    def test_same_token_and_duplicate_publishers_rejected(self):
        config = self.config()
        for change in ({"payout_asset": config["protected_asset"]},
                       {"publisher2": config["publisher1"]}):
            with self.assertRaises(ValueError):
                validate(dict(config, **change))

    def test_secret_key_is_not_accepted_as_identity_alias(self):
        with self.assertRaises(ValueError):
            validate(dict(self.config(), source_account="S"+"A"*55))

    def test_dry_run_cannot_sign_or_invoke_external_commands(self):
        with TemporaryDirectory() as directory:
            config = Path(directory) / "pilot.json"
            config.write_text(json.dumps(self.config()))
            output = io.StringIO()
            with patch("sys.argv", ["deploy_testnet.py", "--config", str(config)]), \
                    patch("deploy_testnet.run", side_effect=AssertionError("must not execute")) as command, \
                    redirect_stdout(output):
                main()
            command.assert_not_called()
            self.assertIn("DRY RUN", output.getvalue())
            self.assertNotIn("zk_verifier", output.getvalue())

    def test_execute_cannot_bypass_network_validation(self):
        with TemporaryDirectory() as directory:
            config = Path(directory) / "pilot.json"
            config.write_text(json.dumps(dict(self.config(), network="mainnet")))
            with patch("sys.argv", ["deploy_testnet.py", "--config", str(config), "--execute"]), \
                    patch("deploy_testnet.run") as command:
                with self.assertRaises(ValueError):
                    main()
            command.assert_not_called()


if __name__ == "__main__":
    unittest.main()
