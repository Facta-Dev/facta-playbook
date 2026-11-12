// <app> — the logger.
//
// Logs are values. They are redacted before they are kept and they land in a
// bounded ring buffer: the worker is mortal, so a buffer beats a file. There
// is no console call anywhere in the extension; the sink is injected.

export type LogLevel = "debug" | "info" | "warn" | "error";

export interface LogEntry {
  readonly level: LogLevel;
  readonly message: string;
  readonly at: string;
  readonly context: Record<string, unknown>;
}

export interface Logger {
  log(level: LogLevel, message: string, context?: Record<string, unknown>): void;
  /** The buffered entries, oldest first. */
  entries(): readonly LogEntry[];
}

export interface LoggerOptions {
  /** How many entries the buffer keeps. */
  readonly capacity?: number;
  /** Where a redacted entry goes, if anywhere. */
  readonly sink?: (entry: LogEntry) => void;
}

const DEFAULT_CAPACITY = 200;

/** A key whose value looks like a secret. */
const SECRET_KEY = /token|secret|password|passwd|authorization|api[_-]?key|cookie|credential/i;

/** A value that carries a credential in the clear. */
const SECRET_VALUE = /\b(bearer|basic)\s+[\w.~+/-]+=*/gi;

export function createLogger(options: LoggerOptions = {}): Logger {
  const capacity = options.capacity ?? DEFAULT_CAPACITY;
  const buffer: LogEntry[] = [];

  return {
    log(level, message, context) {
      const entry: LogEntry = {
        level,
        message,
        at: new Date().toISOString(),
        context: redact(context ?? {}) as Record<string, unknown>,
      };
      buffer.push(entry);
      // ponytail: the buffer lives in memory and dies with the worker. Move
      // it to IndexedDB when diagnostics must survive a restart.
      if (buffer.length > capacity) buffer.splice(0, buffer.length - capacity);
      options.sink?.(entry);
    },
    entries() {
      return [...buffer];
    },
  };
}

function redact(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(redact);
  if (typeof value === "object" && value !== null) {
    const out: Record<string, unknown> = {};
    for (const [key, item] of Object.entries(value as Record<string, unknown>)) {
      out[key] = SECRET_KEY.test(key) ? "[redacted]" : redact(item);
    }
    return out;
  }
  if (typeof value === "string") return value.replace(SECRET_VALUE, "$1 [redacted]");
  return value;
}
