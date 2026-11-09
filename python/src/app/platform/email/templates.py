"""Localized email rendering; unknown templates and placeholders fail loudly."""

from __future__ import annotations

from dataclasses import dataclass

_TEMPLATES: dict[str, dict[str, tuple[str, str]]] = {
    "verify_email": {
        "en": (
            "Verify your email",
            "Your verification code is {code}. It expires in {hours} hours.",
        ),
        "es": (
            "Verifica tu correo",
            "Tu codigo de verificacion es {code}. Expira en {hours} horas.",
        ),
    },
    "reset_password": {
        "en": (
            "Reset your password",
            "Your reset code is {code}. It expires in {minutes} minutes.",
        ),
        "es": (
            "Restablece tu contrasena",
            "Tu codigo de restablecimiento es {code}. Expira en {minutes} minutos.",
        ),
    },
}


@dataclass(frozen=True)
class RenderedEmail:
    """The sender-ready projection of a template."""

    subject: str
    body: str


def render(template_key: str, variables: dict[str, object], locale: str = "en") -> RenderedEmail:
    """Render one template, defaulting to English for unknown locales."""
    translations = _TEMPLATES.get(template_key)
    if translations is None:
        msg = f"unknown email template: {template_key}"
        raise ValueError(msg)
    subject, body = translations.get(locale, translations["en"])
    return RenderedEmail(subject=subject.format(**variables), body=body.format(**variables))
