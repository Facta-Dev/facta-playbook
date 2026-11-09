//! Localized email templates (go `internal/platform/email/templates.go`
//! parity: same keys, langs, and rendered text).
//!
//! ponytail: go renders with `text/template`; the shipped templates only use
//! `{{.name}}` lookups, so a substitution scan replaces a templating crate.
//! Unpaired `{{` survives verbatim; an unknown var name renders empty (go
//! would print `<no value>` — no shipped template references one).

use <app>_core::Error;

/// One rendered mail text: (subject, body), `{{.name}}` substitutions over
/// the vars map.
type MailText = (&'static str, &'static str);
/// A template's per-language table: lang => [`MailText`] (shipped: en + es).
type LangTable = [(&'static str, MailText)];

/// key => per-lang table, copied from go templates.go (en + es).
/// Bodies are `{{.name}}` substitutions over the vars map.
const TEMPLATES: &[(&str, &LangTable)] = &[
    (
        "verify_email",
        &[
            (
                "en",
                (
                    "Verify your email",
                    "Hi {{.first_name}},\n\nUse this code to verify your <app> account:\n\n{{.code}}\n\nIt expires in 24 hours. If you didn't request it, ignore this email.",
                ),
            ),
            (
                "es",
                (
                    "Verifica tu correo",
                    "Hola {{.first_name}},\n\nUsa este código para verificar tu cuenta de <app>:\n\n{{.code}}\n\nExpira en 24 horas. Si no lo solicitaste, ignora este mensaje.",
                ),
            ),
        ],
    ),
    (
        "reset_password",
        &[
            (
                "en",
                (
                    "Reset your password",
                    "Hi {{.first_name}},\n\nUse this code to reset the password of your <app> account:\n\n{{.code}}\n\nIt expires in 30 minutes. If you didn't request it, ignore this email.",
                ),
            ),
            (
                "es",
                (
                    "Restablece tu contraseña",
                    "Hola {{.first_name}},\n\nUsa este código para restablecer la contraseña de tu cuenta de <app>:\n\n{{.code}}\n\nExpira en 30 minutos. Si no lo solicitaste, ignora este mensaje.",
                ),
            ),
        ],
    ),
    (
        "password_changed",
        &[
            (
                "en",
                (
                    "Your password was changed",
                    "Hi {{.first_name}},\n\nThe password of your <app> account was just changed.\nIf it wasn't you, contact support immediately.",
                ),
            ),
            (
                "es",
                (
                    "Tu contraseña fue cambiada",
                    "Hola {{.first_name}},\n\nLa contraseña de tu cuenta de <app> acaba de cambiar.\nSi no fuiste tú, contacta a soporte de inmediato.",
                ),
            ),
        ],
    ),
];

/// Render resolves a template in the user's language (unknown languages fall
/// back to `en`) and substitutes the vars (defaults keep templates safe when a
/// caller omits a var, go Render parity). Unknown keys are template errors.
///
/// # Errors
/// [`Error::Internal`] when `template_key` is not a shipped template.
pub fn render(
    template_key: &str,
    vars: &serde_json::Value,
    lang: &str,
) -> Result<(String, String), Error> {
    let Some((_, by_lang)) = TEMPLATES.iter().find(|(key, _)| *key == template_key) else {
        return Err(Error::Internal(format!(
            "email: unknown template {template_key:?}"
        )));
    };
    let pair = by_lang
        .iter()
        .find(|(l, _)| *l == lang)
        .unwrap_or_else(|| &by_lang[0])
        .1;
    Ok((execute(pair.0, vars), execute(pair.1, vars)))
}

/// go `execute`: substitute `{{.name}}` over defaults (`first_name` "",
/// code "", `app_name` "<app>"), vars winning on collision.
fn execute(raw: &str, vars: &serde_json::Value) -> String {
    let mut data = serde_json::json!({"first_name": "", "code": "", "app_name": "<app>"});
    if let (Some(dst), Some(src)) = (data.as_object_mut(), vars.as_object()) {
        for (key, value) in src {
            dst.insert(key.clone(), value.clone());
        }
    }
    substitute(raw, &data)
}

/// Substitute every `{{.name}}` with `vars[name]` rendered like go `fmt %v`
/// (strings bare, others JSON); unknown/absent => empty string.
fn substitute(raw: &str, vars: &serde_json::Value) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        if let Some(end) = after.find("}}") {
            out.push_str(&lookup(&after[..end], vars));
            rest = &after[end + 2..];
        } else {
            // Unpaired opener: emit it verbatim and scan on from the tail.
            out.push_str("{{");
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

/// One `{{.name}}` lookup: non-`{{.` inners must not be treated as vars (go
/// text/template would treat them as pipeline syntax and error on parse; the
/// shipped templates never contain one, so it is echoed back verbatim).
fn lookup(inner: &str, vars: &serde_json::Value) -> String {
    let Some(name) = inner.trim().strip_prefix('.') else {
        return format!("{{{{{inner}}}}}");
    };
    let name = name.trim();
    match vars.get(name) {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(v @ (serde_json::Value::Number(_) | serde_json::Value::Bool(_))) => v.to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::render;

    #[test]
    fn renders_localized_subject_and_body_vars() {
        // Spanish subject for a Spanish user: language comes from the user row.
        let (subject, body) = render(
            "verify_email",
            &json!({"first_name": "Ana", "code": "123456"}),
            "es",
        )
        .expect("shipped template renders");
        assert!(subject.contains("Verifica"), "es subject: {subject:?}");
        assert!(
            body.contains("123456") && body.contains("Ana"),
            "es body: {body:?}"
        );
    }

    #[test]
    fn unknown_lang_falls_back_to_en() {
        let (subject, _) = render("verify_email", &serde_json::Value::Null, "fr")
            .expect("unknown lang falls back to en");
        assert_eq!(subject, "Verify your email");
    }

    #[test]
    fn unknown_template_key_errors() {
        let err = render("nope", &serde_json::Value::Null, "en")
            .expect_err("unknown template key must error");
        assert!(format!("{err}").contains("unknown template"), "{err}");
    }

    #[test]
    fn defaults_fill_omitted_vars() {
        // app_name falls back to the `<app>` placeholder so rename.sh stamps
        // the real product name; first_name/code default empty.
        let (_, body) =
            render("verify_email", &serde_json::Value::Null, "en").expect("defaults render");
        assert!(
            body.contains("your <app> account"),
            "defaults: {body:?}"
        );
    }
}
