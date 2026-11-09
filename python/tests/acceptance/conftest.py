"""Make the step modules reachable from the scenarios in this folder.

pytest-bdd registers every step as a fixture in the module that defines it,
and pytest only resolves fixtures from the test module and the conftest
chain. Copying the markers into this conftest is what makes a step defined
in ``steps_*.py`` visible to ``test_bdd.py``.
"""

from __future__ import annotations

from tests.acceptance import (
    steps_admin,
    steps_http,
    steps_oauth,
    steps_password,
    steps_session,
    steps_signup,
    steps_widget,
)

_STEP_MODULES = (
    steps_admin,
    steps_http,
    steps_oauth,
    steps_password,
    steps_session,
    steps_signup,
    steps_widget,
)

for _module in _STEP_MODULES:
    for _name, _value in vars(_module).items():
        if _name.startswith("pytestbdd_stepdef_"):
            globals()[_name] = _value
