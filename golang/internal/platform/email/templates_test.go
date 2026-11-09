package email

import (
	"strings"
	"testing"
)

func TestRenderLanguages(t *testing.T) {
	t.Parallel()
	// Spanish subject for a Spanish user: language comes from the user row.
	subject, body, err := Render("verify_email", map[string]any{"first_name": "Ana", "code": "123456", "locale": "es"}, "es")
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(subject, "Verifica") {
		t.Fatalf("es subject: %q", subject)
	}
	if !strings.Contains(body, "123456") || !strings.Contains(body, "Ana") {
		t.Fatalf("es body missing vars: %q", body)
	}
}

func TestRenderUnknownLangFallsBackToEn(t *testing.T) {
	t.Parallel()
	subject, _, err := Render("verify_email", nil, "fr")
	if err != nil {
		t.Fatal(err)
	}
	if subject != "Verify your email" {
		t.Fatalf("en fallback subject: %q", subject)
	}
}

func TestRenderUnknownTemplate(t *testing.T) {
	t.Parallel()
	if _, _, err := Render("nope", nil, "en"); err == nil {
		t.Fatal("want error for unknown template key")
	}
}
