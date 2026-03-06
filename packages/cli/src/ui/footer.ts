/**
 * Persistent terminal footer bar.
 *
 * Reserves the bottom row of the terminal for a context-aware hint line
 * that stays visible across prompt transitions.
 *
 * Strategy (no scroll regions — those are for full-screen apps):
 *   1. Patch `process.stdout.rows` via getter so clack's `getRows()` and
 *      `limitOptions()` see one fewer row. This prevents prompts from
 *      rendering into the footer row.
 *   2. Intercept `stdout.write` — after any write containing `\x1b[J`
 *      (erase-down, emitted by clack on every re-render), repaint the
 *      footer at the physical bottom row.
 *   3. On resize, repaint.
 *   4. On close, reverse all patches cleanly.
 */

import pc from "picocolors";

// ── ANSI sequences ───────────────────────────────────────────────────────────

/** Move cursor to absolute position (1-based row, col). */
const cursorTo = (row: number, col: number) => `\x1b[${row};${col}H`;

const CURSOR_SAVE = "\x1b7";
const CURSOR_RESTORE = "\x1b8";
const ERASE_LINE = "\x1b[2K";

/** Matches sequences that require a footer repaint:
 *  - Erase-down: \x1b[J, \x1b[0J, \x1b[2J (clack emits on every re-render)
 *  - Cursor hide: \x1b[?25l (clack emits on the first render only) */
const REPAINT_RE = /\x1b\[\d*J|\x1b\[\?25l/;

// ── Hint context definitions ─────────────────────────────────────────────────

export type FooterContext =
  | "idle"
  | "multiselect"
  | "select"
  | "confirm"
  | "text"
  | "search"
  | "search-multiselect";

function buildHint(ctx: FooterContext): string {
  const d = pc.dim;
  switch (ctx) {
    case "multiselect":
      return `${d("Space")} toggle  ${d("↑↓")} navigate  ${d("Enter")} confirm  ${d("Ctrl+C")} cancel`;
    case "select":
      return `${d("↑↓")} navigate  ${d("Enter")} confirm  ${d("Ctrl+C")} cancel`;
    case "confirm":
      return `${d("y/n")} choose  ${d("Enter")} confirm  ${d("Ctrl+C")} cancel`;
    case "text":
      return `${d("Enter")} submit  ${d("Ctrl+C")} cancel`;
    case "search":
      return `${d("Type")} search  ${d("↑↓")} navigate  ${d("Enter")} confirm  ${d("Ctrl+C")} cancel`;
    case "search-multiselect":
      return `${d("Type")} search  ${d("Space")} toggle  ${d("↑↓")} navigate  ${d("Enter")} confirm  ${d("Ctrl+C")} cancel`;
    case "idle":
    default:
      return "";
  }
}

// ── FooterBar ────────────────────────────────────────────────────────────────

const FOOTER_HEIGHT = 1;

export class FooterBar {
  private _active = false;
  private _context: FooterContext = "idle";
  private _output: NodeJS.WriteStream;
  private _resizeHandler: (() => void) | null = null;
  private _exitHandler: (() => void) | null = null;
  private _realRowsDescriptor: PropertyDescriptor | undefined;
  private _snapshotRows = 24;
  private _originalWrite: NodeJS.WriteStream["write"] | null = null;
  private _painting = false;

  constructor(output: NodeJS.WriteStream = process.stdout) {
    this._output = output;
  }

  // ── Public API ────────────────────────────────────────────────────────────

  open(): void {
    if (this._active) return;
    if (!this._output.isTTY) return;

    this._active = true;

    // ── Patch rows ──────────────────────────────────────────────────────────
    this._realRowsDescriptor = Object.getOwnPropertyDescriptor(
      this._output,
      "rows",
    );
    this._snapshotRows = (this._output as any).rows ?? 24;
    this._installRowsGetter();

    // ── Patch write ─────────────────────────────────────────────────────────
    this._originalWrite = this._output.write.bind(this._output);
    const self = this;
    this._output.write = function interceptedWrite(
      chunk: any,
      encodingOrCb?: any,
      cb?: any,
    ): boolean {
      const result = self._originalWrite!.call(
        self._output,
        chunk,
        encodingOrCb,
        cb,
      );

      // Repaint footer after erase-down (clack emits \x1b[J on every render)
      if (!self._painting && self._active && self._context !== "idle") {
        const str =
          typeof chunk === "string" ? chunk : chunk?.toString?.() ?? "";
        if (REPAINT_RE.test(str)) {
          self._paintFooter();
        }
      }

      return result;
    } as any;

    // ── Initial paint ───────────────────────────────────────────────────────
    this._paintFooter();

    // ── Resize ──────────────────────────────────────────────────────────────
    this._resizeHandler = () => {
      if (!this._active) return;
      const fresh = Object.getOwnPropertyDescriptor(this._output, "rows");
      if (fresh && "value" in fresh) {
        this._snapshotRows = fresh.value ?? this._snapshotRows;
        this._installRowsGetter();
      }
      this._paintFooter();
    };
    this._output.on("resize", this._resizeHandler);

    // ── Exit cleanup ────────────────────────────────────────────────────────
    this._exitHandler = () => this.close();
    process.on("exit", this._exitHandler);
  }

  setContext(ctx: FooterContext): void {
    if (this._context === ctx) return;
    const wasIdle = this._context === "idle";
    this._context = ctx;
    // When activating from idle, don't paint eagerly — the write interceptor
    // will paint after the first prompt frame renders (cursor-hide trigger).
    // Painting before the frame causes overlap when the cursor is near the
    // terminal bottom. For all other transitions, paint immediately.
    if (this._active && !wasIdle) this._paintFooter();
  }

  close(): void {
    if (!this._active) return;
    this._active = false;

    if (this._resizeHandler) {
      this._output.off("resize", this._resizeHandler);
      this._resizeHandler = null;
    }
    if (this._exitHandler) {
      process.off("exit", this._exitHandler);
      this._exitHandler = null;
    }

    // Restore original write BEFORE writing cleanup sequences
    if (this._originalWrite) {
      this._output.write = this._originalWrite as any;
      this._originalWrite = null;
    }

    // Restore real rows property
    if (this._realRowsDescriptor) {
      Object.defineProperty(this._output, "rows", this._realRowsDescriptor);
    } else {
      delete (this._output as any).rows;
    }

    // Clear the footer row and move cursor there so the shell prompt is clean
    const total = this._realRowsValue();
    this._output.write(
      `${CURSOR_SAVE}${cursorTo(total, 1)}${ERASE_LINE}${CURSOR_RESTORE}`,
    );
  }

  get isActive(): boolean {
    return this._active;
  }

  // ── Internal ──────────────────────────────────────────────────────────────

  private _installRowsGetter(): void {
    const self = this;
    Object.defineProperty(this._output, "rows", {
      get() {
        const real = self._realRowsValue();
        return self._active ? Math.max(real - FOOTER_HEIGHT, 3) : real;
      },
      set(value: number) {
        // Bun's TTY _refreshSize assigns this.rows directly — without a
        // setter the assignment throws "Attempted to assign to readonly
        // property". Capture the new real value so the getter stays correct.
        self._snapshotRows = value;
      },
      configurable: true,
      enumerable: true,
    });
  }

  private _realRowsValue(): number {
    const desc = this._realRowsDescriptor;
    if (desc) {
      if (desc.get) return desc.get.call(this._output) ?? 24;
      if (desc.value != null) return desc.value;
    }
    return this._snapshotRows;
  }

  private _paintFooter(): void {
    if (this._painting) return;
    this._painting = true;

    const total = this._realRowsValue();
    const hint = buildHint(this._context);
    const w = this._originalWrite ?? this._output.write.bind(this._output);

    if (hint) {
      w.call(
        this._output,
        `${CURSOR_SAVE}${cursorTo(total, 1)}${ERASE_LINE} ${hint}${CURSOR_RESTORE}`,
      );
    } else {
      w.call(
        this._output,
        `${CURSOR_SAVE}${cursorTo(total, 1)}${ERASE_LINE}${CURSOR_RESTORE}`,
      );
    }

    this._painting = false;
  }
}

// ── Singleton ────────────────────────────────────────────────────────────────

let _instance: FooterBar | null = null;

export function footer(): FooterBar {
  if (!_instance) {
    _instance = new FooterBar();
  }
  return _instance;
}

// ── Footer-aware clack wrappers ─────────────────────────────────────────────
// Thin wrappers that set the footer context before calling the real clack
// function. After the prompt resolves (submit or cancel), context resets to
// idle so the footer clears between prompts.

import {
  confirm as _confirm,
  multiselect as _multiselect,
  select as _select,
  text as _text,
} from "@clack/prompts";

type SelectParams = Parameters<typeof _select>[0];
type MultiselectParams = Parameters<typeof _multiselect>[0];
type ConfirmParams = Parameters<typeof _confirm>[0];
type TextParams = Parameters<typeof _text>[0];

function withFooter<T>(ctx: FooterContext, fn: () => T): T {
  const f = footer();
  if (f.isActive) f.setContext(ctx);
  const result = fn();
  // If the fn returns a promise (all clack prompts do), reset after it settles
  if (result && typeof (result as any).then === "function") {
    (result as any).then(
      () => { if (f.isActive) f.setContext("idle"); },
      () => { if (f.isActive) f.setContext("idle"); },
    );
  }
  return result;
}

export function footerSelect<Value>(opts: SelectParams): ReturnType<typeof _select<Value>> {
  return withFooter("select", () => _select<Value>(opts));
}

export function footerMultiselect<Value>(opts: MultiselectParams): ReturnType<typeof _multiselect<Value>> {
  return withFooter("multiselect", () => _multiselect<Value>(opts));
}

export function footerConfirm(opts: ConfirmParams): ReturnType<typeof _confirm> {
  return withFooter("confirm", () => _confirm(opts));
}

export function footerText(opts: TextParams): ReturnType<typeof _text> {
  return withFooter("text", () => _text(opts));
}
