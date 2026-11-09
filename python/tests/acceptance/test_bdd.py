"""Acceptance scenarios: the real app under uvicorn, fake adapters."""

from __future__ import annotations

from pytest_bdd import scenarios

from tests.conftest import FEATURES

scenarios(str(FEATURES / "authsignup.feature"))
scenarios(str(FEATURES / "authsession.feature"))
scenarios(str(FEATURES / "session-mgmt.feature"))
scenarios(str(FEATURES / "authpassword.feature"))
scenarios(str(FEATURES / "authoauth.feature"))
scenarios(str(FEATURES / "adminusers.feature"))
scenarios(str(FEATURES / "widgets.feature"))
