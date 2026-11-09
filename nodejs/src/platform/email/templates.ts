// Localized email templates: key => lang => {subject, body}; bodies are
// {{.code}}-style vars substituted in Render. Port of the Go template's
// email/templates.go.

import type { MessageVars } from "./queue.js";

// Language defaults: codes come from the users.language column; anything
// unknown falls back to English.
const defaultLang = "en";

type Pair = readonly [subject: string, body: string];

// templates: key => lang => {subject, body}.
const templates: Record<string, Record<string, Pair>> = {
  verify_email: {
    en: [
      "Verify your email",
      "Hi {{first_name}},\n\nUse this code to verify your {{app_name}} account:\n\n{{code}}\n\nIt expires in 24 hours. If you didn't request it, ignore this email.",
    ],
    es: [
      "Verifica tu correo",
      "Hola {{first_name}},\n\nUsa este código para verificar tu cuenta de {{app_name}}:\n\n{{code}}\n\nExpira en 24 horas. Si no lo solicitaste, ignora este mensaje.",
    ],
  },
  reset_password: {
    en: [
      "Reset your password",
      "Hi {{first_name}},\n\nUse this code to reset the password of your {{app_name}} account:\n\n{{code}}\n\nIt expires in 30 minutes. If you didn't request it, ignore this email.",
    ],
    es: [
      "Restablece tu contraseña",
      "Hola {{first_name}},\n\nUsa este código para restablecer la contraseña de tu cuenta de {{app_name}}:\n\n{{code}}\n\nExpira en 30 minutos. Si no lo solicitaste, ignora este mensaje.",
    ],
  },
  password_changed: {
    en: [
      "Your password was changed",
      "Hi {{first_name}},\n\nThe password of your {{app_name}} account was just changed.\nIf it wasn't you, contact support immediately.",
    ],
    es: [
      "Tu contraseña fue cambiada",
      "Hola {{first_name}},\n\nLa contraseña de tu cuenta de {{app_name}} acaba de cambiar.\nSi no fuiste tú, contacta a soporte de inmediato.",
    ],
  },
};

// Render resolves a template in the user's language (unknown languages fall
// back to en) and substitutes the vars; unknown keys are errors.
export function render(
  templateKey: string,
  vars: MessageVars,
  lang: string,
): { ok: true; subject: string; body: string } | { ok: false; err: Error } {
  const byLang = templates[templateKey];
  if (byLang === undefined) {
    return { ok: false, err: new Error(`email: unknown template ${JSON.stringify(templateKey)}`) };
  }
  const pair = byLang[lang] ?? byLang[defaultLang]!;
  // Defaults keep templates safe when a slice omits a var.
  const data: MessageVars = { first_name: "", code: "", app_name: "<app>", ...(vars ?? {}) };
  return {
    ok: true,
    subject: execute(pair[0], data),
    body: execute(pair[1], data),
  };
}

const execute = (raw: string, data: MessageVars): string =>
  raw.replace(/\{\{(\w+)\}\}/g, (_m, k: string) => String(data[k] ?? ""));
