package email

import (
	"context"
	"log/slog"
	"net"
	"net/smtp"
	"strconv"

	"github.com/<scope>/<app>/internal/platform/config"
)

// Sender delivers one rendered email.
type Sender interface {
	Send(ctx context.Context, to, subject, body string) error
}

// ConsoleSender logs instead of dialing any socket: dev, tests and the
// INV-T7 offline guard (<APP>_OFFLINE=true).
type ConsoleSender struct {
	log *slog.Logger
}

// NewConsoleSender builds the console sender.
func NewConsoleSender(log *slog.Logger) ConsoleSender { return ConsoleSender{log: log} }

func (s ConsoleSender) Send(_ context.Context, to, subject, _ string) error {
	s.log.Info("email.console", "to", to, "subject", subject, "sender", "console")
	return nil
}

// SMTPSender sends via net/smtp (STARTTLS is negotiated automatically when
// the server offers it; the template keeps no TLS-complexity).
type SMTPSender struct {
	host string // host:port
	from string
	auth smtp.Auth
}

// NewSMTPSender builds the sender from config (any empty user => no auth).
func NewSMTPSender(cfg config.Config) *SMTPSender {
	var auth smtp.Auth
	if cfg.SMTPUser != "" {
		auth = smtp.PlainAuth("", cfg.SMTPUser, cfg.SMTPPass, cfg.SMTPHost)
	}
	return &SMTPSender{host: net.JoinHostPort(cfg.SMTPHost, strconv.Itoa(cfg.SMTPPort)), from: cfg.SMTPFrom, auth: auth}
}

func (s *SMTPSender) Send(_ context.Context, to, subject, body string) error {
	// ponytail: net/smtp has no context plumbing; long SMTP hangs are bounded
	// server-side, not here — swap to a context-aware client if that bites.
	msg := "From: " + s.from + "\r\n" +
		"To: " + to + "\r\n" +
		"Subject: " + subject + "\r\n" +
		"MIME-Version: 1.0\r\n" +
		"Content-Type: text/plain; charset=\"utf-8\"\r\n\r\n" +
		body
	return smtp.SendMail(s.host, s.auth, s.from, []string{to}, []byte(msg))
}

// SelectSender honors the offline guard first, then configured SMTP.
func SelectSender(cfg config.Config, log *slog.Logger) Sender {
	if cfg.Offline || cfg.SMTPHost == "" {
		return NewConsoleSender(log)
	}
	return NewSMTPSender(cfg)
}
