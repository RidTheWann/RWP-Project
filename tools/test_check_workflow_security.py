#!/usr/bin/env python3
"""Unit tests for the CI workflow security invariant checker."""

from __future__ import annotations

import unittest

from check_workflow_security import check_workflow


VALID_WORKFLOW = """name: CI

permissions:
  contents: read

jobs:
  gate:
    steps:
      - name: Require audited commit
        env:
          HEAD_REF: ${{ github.head_ref }}
          BASE_REF: ${{ github.base_ref }}
          REF_NAME: ${{ github.ref_name }}
        run: |
          test "$HEAD_REF" = development
          test "$BASE_REF" = main
          test "$REF_NAME" = development
"""


class WorkflowSecurityTests(unittest.TestCase):
    def test_accepts_environment_passed_refs_and_read_only_token(self) -> None:
        self.assertEqual(check_workflow(VALID_WORKFLOW), [])

    def test_rejects_interpolated_branch_ref_in_shell(self) -> None:
        workflow = VALID_WORKFLOW.replace(
            'test "$HEAD_REF" = development',
            'test "${{ github.head_ref }}" = development',
        )
        violations = check_workflow(workflow)
        self.assertTrue(any("interpolates" in item for item in violations))

    def test_rejects_write_scopes_not_used_by_ci(self) -> None:
        workflow = VALID_WORKFLOW.replace(
            "  contents: read",
            "  contents: read\n  actions: write",
        )
        violations = check_workflow(workflow)
        self.assertTrue(any("actions: write" in item for item in violations))

    def test_rejects_missing_branch_ref_environment_mapping(self) -> None:
        workflow = VALID_WORKFLOW.replace(
            "          BASE_REF: ${{ github.base_ref }}\n",
            "",
        )
        violations = check_workflow(workflow)
        self.assertTrue(any("BASE_REF" in item for item in violations))

    def test_rejects_full_history_checkout(self) -> None:
        workflow = VALID_WORKFLOW.replace(
            "    steps:\n",
            "    steps:\n      - uses: actions/checkout@v7\n        with:\n          fetch-depth: 0\n",
        )
        violations = check_workflow(workflow)
        self.assertTrue(any("full history" in item for item in violations))


if __name__ == "__main__":
    unittest.main(verbosity=2)
