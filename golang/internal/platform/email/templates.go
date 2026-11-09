package email

import (
	"bytes"
	"fmt"
	"text/template"
)

// Language defaults: codes come from the users.language column; anything
// unknown falls back to English.
const defaultLang = "en"

// templates: key => lang => {subject, body}. Bodies are text/template with
// named vars ({{.code}}, {{.first_name}}, {{.app_name}}).
var templates = map[string]map[string][2]string{
	"verify_email": {
		"en": {"Verify your email",
			"Hi {{.first_name}},\n\nUse this code to verify your {{.app_name}} account:\n\n{{.code}}\n\nIt expires in 24 hours. If you didn't request it, ignore this email."},
		"es": {"Verifica tu correo",
			"Hola {{.first_name}},\n\nUsa este código para verificar tu cuenta de {{.app_name}}:\n\n{{.code}}\n\nExpira en 24 horas. Si no lo solicitaste, ignora este mensaje."},
	},
	"reset_password": {
		"en": {"Reset your password",
			"Hi {{.first_name}},\n\nUse this code to reset the password of your {{.app_name}} account:\n\n{{.code}}\n\nIt expires in 30 minutes. If you didn't request it, ignore this email."},
		"es": {"Restablece tu contraseña",
			"Hola {{.first_name}},\n\nUsa este código para restablecer la contraseña de tu cuenta de {{.app_name}}:\n\n{{.code}}\n\nExpira en 30 minutos. Si no lo solicitaste, ignora este mensaje."},
	},
	"password_changed": {
		"en": {"Your password was changed",
			"Hi {{.first_name}},\n\nThe password of your {{.app_name}} account was just changed.\nIf it wasn't you, contact support immediately."},
		"es": {"Tu contraseña fue cambiada",
			"Hola {{.first_name}},\n\nLa contraseña de tu cuenta de {{.app_name}} acaba de cambiar.\nSi no fuiste tú, contacta a soporte de inmediato."},
	},
}

// Render resolves a template in the user's language (unknown languages fall
// back to en) and substitutes the vars; unknown keys are templates errors.
func Render(templateKey string, vars map[string]any, lang string) (subject, body string, err error) {
	byLang, ok := templates[templateKey]
	if !ok {
		return "", "", fmt.Errorf("email: unknown template %q", templateKey)
	}
	pair, ok := byLang[lang]
	if !ok {
		pair = byLang[defaultLang]
	}
	// Defaults keep templates safe when a slice omits a var.
	data := map[string]any{"first_name": "", "code": "", "app_name": "<app>"}
	for k, v := range vars {
		data[k] = v
	}
	subject, err = execute(pair[0], data)
	if err != nil {
		return "", "", fmt.Errorf("email %s: %w", templateKey, err)
	}
	body, err = execute(pair[1], data)
	if err != nil {
		return "", "", fmt.Errorf("email %s: %w", templateKey, err)
	}
	return subject, body, nil
}

func execute(raw string, data map[string]any) (string, error) {
	t, err := template.New("mail").Parse(raw)
	if err != nil {
		return "", err
	}
	var buf bytes.Buffer
	if err := t.Execute(&buf, data); err != nil {
		return "", err
	}
	return buf.String(), nil
}
