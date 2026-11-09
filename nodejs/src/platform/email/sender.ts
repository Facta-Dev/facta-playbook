// Sender implementations: console (dev/tests/offline — INV-T7) and SMTP via
// nodemailer. Port of the Go template's email/sender.go.

import { type Transporter, createTransport } from "nodemailer";

// Sender delivers one rendered email.
export interface Sender {
  send(to: string, subject: string, body: string): Promise<void>;
}

// ConsoleSender logs instead of dialing any socket: dev, tests and the
// INV-T7 offline guard (<APP>_OFFLINE=true).
export class ConsoleSender implements Sender {
  constructor(private readonly log: { info: (o: object) => void }) {}

  async send(to: string, subject: string, _body: string): Promise<void> {
    this.log.info({ sender: "console", to, subject });
  }
}

// SMTPSender sends via nodemailer (one send per connection, matching the Go
// template's stateless SendMail).
export class SMTPSender implements Sender {
  private readonly transport: Transporter;
  private readonly from: string;

  constructor(cfg: {
    smtpHost: string;
    smtpPort: number;
    smtpUser: string;
    smtpPass: string;
    smtpFrom: string;
  }) {
    this.from = cfg.smtpFrom;
    this.transport = createTransport({
      host: cfg.smtpHost,
      port: cfg.smtpPort,
      secure: cfg.smtpPort === 465,
      ...(cfg.smtpUser !== "" ? { auth: { user: cfg.smtpUser, pass: cfg.smtpPass } } : {}),
    });
  }

  async send(to: string, subject: string, body: string): Promise<void> {
    await this.transport.sendMail({ from: this.from, to, subject, text: body });
  }
}

// SelectSender honors the offline guard first, then configured SMTP.
export function selectSender(
  cfg: {
    offline: boolean;
    smtpHost: string;
    smtpPort: number;
    smtpUser: string;
    smtpPass: string;
    smtpFrom: string;
  },
  log: { info: (o: object) => void },
): Sender {
  if (cfg.offline || cfg.smtpHost === "") return new ConsoleSender(log);
  return new SMTPSender(cfg);
}
