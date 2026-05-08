export type OutputMode = "tty" | "plain" | "json";

export interface OutputOptions {
  json?: boolean;
  quiet?: boolean;
  isTTY?: boolean;
  stdout?: NodeJS.WritableStream;
  stderr?: NodeJS.WritableStream;
}

export interface Progress {
  update(message: string): void;
  succeed(message?: string): void;
  fail(message?: string): void;
  pause(): void;
  resume(): void;
}

export interface Output {
  readonly mode: OutputMode;

  info(message: string): void;
  warn(message: string, hint?: string): void;
  error(message: string, hint?: string): void;
  success(message: string): void;
  block(lines: string[], opts?: { stream?: "stdout" | "stderr" }): void;

  json<T>(event: T): void;

  progress(label: string): Progress;

  raw(text: string): void;
}
