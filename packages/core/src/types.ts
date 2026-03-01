export type Result<T, E = SkilltapError> =
  | { ok: true; value: T }
  | { ok: false; error: E };

export function ok<T>(value: T): { ok: true; value: T } {
  return { ok: true, value };
}

export function err<E>(error: E): { ok: false; error: E } {
  return { ok: false, error };
}

export class SkilltapError extends Error {
  hint?: string;

  constructor(message: string, hint?: string) {
    super(message);
    this.name = "SkilltapError";
    this.hint = hint;
  }
}

export class UserError extends SkilltapError {
  constructor(message: string, hint?: string) {
    super(message, hint);
    this.name = "UserError";
  }
}

export class GitError extends SkilltapError {
  constructor(message: string, hint?: string) {
    super(message, hint);
    this.name = "GitError";
  }
}

export class ScanError extends SkilltapError {
  constructor(message: string, hint?: string) {
    super(message, hint);
    this.name = "ScanError";
  }
}

export class NetworkError extends SkilltapError {
  constructor(message: string, hint?: string) {
    super(message, hint);
    this.name = "NetworkError";
  }
}

export const EXIT_SUCCESS = 0;
export const EXIT_ERROR = 1;
export const EXIT_CANCELLED = 2;
