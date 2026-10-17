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
        cls.css = (WEB / "styles.css").read_text(encoding="utf-8")
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

    def test_stability_is_not_inferred_from_stutter_count(self) -> None:
        self.assertIn('const status = stable ? "stable" : "unknown";', self.js)
        self.assertNotIn('stutters.length>0 ? "unstable"', self.js)
        self.assertIn("const rawFrameCount = snapshot.frame_count;", self.js)
        self.assertIn("const rawStutters = dashboard.stutters;", self.js)
        self.assertIn('frameCount === null ? t("noData")', self.js)
        self.assertIn('stutters === null', self.js)

    def _css_var(self, name: str) -> str:
        match = re.search(rf"--{re.escape(name)}:\s*(#[0-9A-Fa-f]{{6}})", self.css)
        self.assertIsNotNone(match, f"CSS variable --{name} must be a six-digit color")
        return match.group(1)

    @staticmethod
    def _luminance(color: str) -> float:
        channels = [int(color.lstrip("#")[index:index + 2], 16) / 255 for index in (0, 2, 4)]
        linear = [
            channel / 12.92 if channel <= 0.04045 else ((channel + 0.055) / 1.055) ** 2.4
            for channel in channels
        ]
        return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2]

    @classmethod
    def _contrast_ratio(cls, foreground: str, background: str) -> float:
        high, low = sorted(
            (cls._luminance(foreground), cls._luminance(background)),
            reverse=True,
        )
        return (high + 0.05) / (low + 0.05)

    def test_primary_text_and_hover_colors_meet_wcag_aa_contrast(self) -> None:
        text = self._css_var("text")
        muted = self._css_var("muted")
        background = self._css_var("bg")
        surface = self._css_var("surface")
        pairs = [
            ("text on background", text, background, 4.5),
            ("muted text on background", muted, background, 4.5),
            ("text on panel", text, surface, 4.5),
            ("muted text on panel", muted, surface, 4.5),
            ("button text on accent", "#FFFFFF", self._css_var("accent"), 4.5),
            ("button text on hover", "#FFFFFF", self._css_var("accent-hover"), 4.5),
            ("focus indicator on background", self._css_var("focus"), background, 3.0),
            ("focus indicator on panel", self._css_var("focus"), surface, 3.0),
        ]
        for label, foreground, backdrop, minimum in pairs:
            with self.subTest(pair=label):
                self.assertGreaterEqual(
                    self._contrast_ratio(foreground, backdrop),
                    minimum,
                    f"{label} must meet its contrast minimum",
                )

    def test_language_refresh_preserves_the_active_view(self) -> None:
        apply_locale = self.js.split("function applyLocale()", 1)[1].split("function escapeHtml", 1)[0]
        self.assertIn("const currentView = state.view;", apply_locale)
        self.assertIn("if (state.view !== currentView) setView(currentView);", apply_locale)

    def test_comparison_pair_change_requires_a_new_scenario_answer(self) -> None:
        self.assertIn("const oldBefore = state.beforeId, oldAfter = state.afterId;", self.js)
        self.assertIn("if (oldBefore !== state.beforeId || oldAfter !== state.afterId)", self.js)
        self.assertIn("input.checked = false;", self.js)
        self.assertIn("el.compareResult.hidden = true;", self.js)

    def test_close_event_permission_is_explicit(self) -> None:
        self.assertIn("core:event:default", self.capability["permissions"])
        self.assertIn("rwp-window-close-requested", self.js)
        self.assertIn(
            "rwp-window-close-requested",
            (TAURI / "src" / "main.rs").read_text(encoding="utf-8"),
        )

    def test_allowlist_equals_ui_invoked_commands(self) -> None:
        toml_text = (TAURI / "permissions" / "default.toml").read_text(encoding="utf-8")
        match = re.search(r"commands\.allow = \[(?P<body>.*?)\]", toml_text, flags=re.DOTALL)
        self.assertIsNotNone(match, "session-commands allowlist is missing")
        allowed = set(re.findall(r'"(session_\w+)"', match.group("body")))
        invoked = set(re.findall(r'invoke\("(\w+)"', self.js))
        invoked.update(re.findall(r"invoke\('(\w+)'", self.js))
        self.assertEqual(
            sorted(allowed),
            sorted(invoked),
            "webview grant must equal exactly the invoked command set (least privilege)",
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
