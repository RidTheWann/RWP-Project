#!/usr/bin/env python3
"""Static regression checks for the native Tauri webview contract."""

from __future__ import annotations

import json
import re
import unittest
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / "crates" / "rwp-ui-native" / "web"
TAURI = ROOT / "crates" / "rwp-ui-native" / "src-tauri"


class MarkupInventory(HTMLParser):
    """Collect IDs and asset references without requiring a browser package."""

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.ids: set[str] = set()
        self.assets: list[str] = []
        self.details_depth = 0
        self.presentmon_path_is_advanced = False

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        values = dict(attrs)
        element_id = values.get("id")
        if element_id:
            self.ids.add(element_id)
        if tag == "details":
            self.details_depth += 1
        if tag == "script" and values.get("src"):
            self.assets.append(str(values["src"]))
        if tag == "link" and values.get("href") and values.get("rel") == "stylesheet":
            self.assets.append(str(values["href"]))
        if element_id == "presentmon-path":
            self.presentmon_path_is_advanced = self.details_depth > 0

    def handle_endtag(self, tag: str) -> None:
        if tag == "details":
            self.details_depth = max(0, self.details_depth - 1)


class UiContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.html = (WEB / "index.html").read_text(encoding="utf-8")
        cls.js = (WEB / "app.js").read_text(encoding="utf-8")
        cls.commands = (TAURI / "src" / "commands.rs").read_text(encoding="utf-8")
        cls.capability = json.loads(
            (TAURI / "capabilities" / "default.json").read_text(encoding="utf-8")
        )
        cls.tauri_config = json.loads(
            (TAURI / "tauri.conf.json").read_text(encoding="utf-8")
        )
        cls.markup = MarkupInventory()
        cls.markup.feed(cls.html)

    def test_literal_javascript_dom_ids_exist_in_markup(self) -> None:
        references = set(re.findall(r'\$\("([^"]+)"\)', self.js))
        references.update(re.findall(r'getElementById\("([^"]+)"\)', self.js))
        self.assertEqual(
            sorted(references - self.markup.ids),
            [],
            "JavaScript references DOM IDs that are not present in index.html",
        )

    def test_webview_assets_are_local_only(self) -> None:
        remote = [
            asset
            for asset in self.markup.assets
            if urlparse(asset).scheme or asset.startswith("//")
        ]
        self.assertEqual(remote, [], "the native shell must not load remote assets")
        self.assertEqual(
            self.tauri_config["app"]["security"]["csp"],
            "default-src 'self'",
        )

    def test_presentmon_path_stays_inside_advanced_disclosure(self) -> None:
        self.assertTrue(
            self.markup.presentmon_path_is_advanced,
            "the executable path is a technical setting and must stay behind Advanced",
        )

    def test_managed_capture_request_matches_backend_fields(self) -> None:
        match = re.search(
            r"pub struct ManagedSessionStartRequest\s*\{(?P<body>.*?)\n\}",
            self.commands,
            flags=re.DOTALL,
        )
        self.assertIsNotNone(match, "managed capture request type is missing")
        fields = set(re.findall(r"pub (\w+):", match.group("body")))
        call = self.js.split('invoke("session_start_managed"', 1)
        self.assertEqual(len(call), 2, "the GUI must use the managed capture command")
        payload = call[1].split("catch", 1)[0]
        sent_fields = set(re.findall(r"\b(\w+):", payload))
        required_fields = fields - {"stop_existing_session"}
        self.assertTrue(
            required_fields.issubset(sent_fields),
            f"missing request fields: {required_fields - sent_fields}",
        )

    def test_close_event_permission_is_explicit(self) -> None:
        self.assertIn("core:event:default", self.capability["permissions"])
        self.assertIn("rwp-window-close-requested", self.js)
        self.assertIn(
            "rwp-window-close-requested",
            (TAURI / "src" / "main.rs").read_text(encoding="utf-8"),
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
