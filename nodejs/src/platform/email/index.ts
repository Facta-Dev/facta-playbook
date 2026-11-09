// Platform email barrel: message/queue types, outbox adapter, senders and
// localized templates (port of the Go template's email package).
export {
  type Job,
  type Message,
  type MessageVars,
  Outbox,
  type Queue,
  drain,
  runEmailWorker,
} from "./queue.js";
export { ConsoleSender, type Sender, SMTPSender, selectSender } from "./sender.js";
export { render } from "./templates.js";
