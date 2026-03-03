/**
 * Reactive search prompt built on @clack/core's Prompt base class.
 *
 * Combines async data fetching (debounced, with AbortSignal) with fzf fuzzy
 * matching for instant local re-ranking. Matches clack's visual style exactly.
 */

import { Prompt, getColumns, getRows } from "@clack/core";
import {
  limitOptions,
  S_BAR,
  S_BAR_END,
  S_RADIO_ACTIVE,
  S_RADIO_INACTIVE,
  symbol,
} from "@clack/prompts";
import { Fzf, byLengthAsc } from "fzf";
import type { FzfResultItem } from "fzf";
import pc from "picocolors";
import type { Key } from "node:readline";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface SearchPromptOptions<T> {
  message: string;
  placeholder?: string;
  /** Called on every query change (debounced for async). Return items to display. */
  source: (query: string, signal: AbortSignal) => T[] | Promise<T[]>;
  /** Render a single option line. `positions` contains fzf match char indices. */
  renderItem: (
    item: T,
    active: boolean,
    matchPositions?: Set<number>,
  ) => string;
  /** String selector for fzf matching. Defaults to `String(item)`. */
  selector?: (item: T) => string;
  /** Debounce ms for async sources. Default 250. Sync sources are never debounced. */
  debounce?: number;
  /** Pre-fill search input. Triggers immediate source fetch. */
  initialQuery?: string;
  /** Max visible options before scrolling. */
  maxItems?: number;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SPINNER_FRAMES = ["◒", "◐", "◓", "◑"];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function isThenable(value: unknown): value is PromiseLike<unknown> {
  return value != null && typeof (value as any).then === "function";
}

function wrapAsResults<T>(items: T[]): FzfResultItem<T>[] {
  return items.map((item) => ({
    item,
    score: 0,
    start: -1,
    end: -1,
    positions: new Set<number>(),
  }));
}

// ---------------------------------------------------------------------------
// SearchPrompt class
// ---------------------------------------------------------------------------

class SearchPromptImpl<T> extends Prompt<T> {
  private opts: SearchPromptOptions<T>;

  // Data
  private allItems: T[] = [];
  private fzfResults: FzfResultItem<T>[] = [];
  private fzfInstance: Fzf<T> | null = null;
  private optionsCursor = 0;

  // Async state
  private isLoading = false;
  private spinnerFrame = 0;
  private isAsyncSource: boolean | null = null; // null = not yet determined
  private abortController: AbortController | null = null;
  private debounceTimer: ReturnType<typeof setTimeout> | null = null;
  private spinnerInterval: ReturnType<typeof setInterval> | null = null;

  // Re-render trigger (Prompt.render() is private in TS but exists at runtime)
  private rerender: () => void;

  constructor(opts: SearchPromptOptions<T>) {
    // We need to bind renderFrame before passing to super, but `this` isn't
    // available yet. Use a closure that defers to the instance.
    let instance: SearchPromptImpl<T>;
    super(
      {
        render() {
          return instance.renderFrame();
        },
        initialUserInput: opts.initialQuery,
        validate: (value: T | undefined) => {
          if (value === undefined) return "No skill selected";
        },
      },
      true, // trackValue: sync userInput with readline
    );
    instance = this;
    this.opts = opts;

    // Grab the bound render method from the base class (private in TS, exists at runtime)
    this.rerender = (this as any).render.bind(this);

    // Event handlers
    this.on("userInput", (input: string) => this.handleInput(input));

    this.on("cursor", (direction) => {
      if (direction === "up") {
        if (this.fzfResults.length > 0) {
          this.optionsCursor =
            this.optionsCursor > 0
              ? this.optionsCursor - 1
              : this.fzfResults.length - 1;
          this._setValue(this.fzfResults[this.optionsCursor]?.item);
        }
      } else if (direction === "down") {
        if (this.fzfResults.length > 0) {
          this.optionsCursor =
            this.optionsCursor < this.fzfResults.length - 1
              ? this.optionsCursor + 1
              : 0;
          this._setValue(this.fzfResults[this.optionsCursor]?.item);
        }
      }
    });

    this.on("finalize", () => this.cleanup());
  }

  override prompt(): Promise<symbol | T | undefined> {
    const result = super.prompt();
    // Always trigger an initial fetch. When initialQuery is set, the base
    // class fires userInput which may have already called scheduleFetch, but
    // calling it again just aborts+restarts (idempotent).
    this.scheduleFetch(this.userInput || "");
    return result;
  }

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  protected _isActionKey(char: string | undefined, _key: Key): boolean {
    // Only tab is an "action key" (deleted from readline buffer).
    // Arrow keys are escape sequences and don't insert into readline.
    return char === "\t";
  }

  // ---------------------------------------------------------------------------
  // Input & source fetching
  // ---------------------------------------------------------------------------

  private handleInput(input: string): void {
    // Instant local re-rank with fzf
    this.applyFzf(input);

    // Debounced source refetch
    this.scheduleFetch(input);
  }

  private scheduleFetch(query: string): void {
    // Abort previous
    if (this.abortController) this.abortController.abort();
    if (this.debounceTimer) clearTimeout(this.debounceTimer);

    const controller = new AbortController();
    this.abortController = controller;

    if (this.isAsyncSource === false) {
      // Known sync source — call immediately
      this.executeFetch(query, controller);
      return;
    }

    if (this.isAsyncSource === null) {
      // First call — detect sync vs async
      this.executeFetch(query, controller);
      return;
    }

    // Known async — debounce
    this.setLoading(true);
    this.debounceTimer = setTimeout(() => {
      this.executeFetch(query, controller);
    }, this.opts.debounce ?? 250);
  }

  private executeFetch(query: string, controller: AbortController): void {
    let result: T[] | Promise<T[]>;
    try {
      result = this.opts.source(query, controller.signal);
    } catch {
      return;
    }

    if (isThenable(result)) {
      // Async path
      if (this.isAsyncSource === null) this.isAsyncSource = true;
      this.setLoading(true);
      (result as Promise<T[]>)
        .then((items) => {
          if (controller.signal.aborted) return; // Stale
          this.receiveItems(items);
          this.setLoading(false);
        })
        .catch(() => {
          if (controller.signal.aborted) return;
          this.setLoading(false);
          this.rerender();
        });
    } else {
      // Sync path
      if (this.isAsyncSource === null) this.isAsyncSource = false;
      this.receiveItems(result as T[]);
    }
  }

  private receiveItems(items: T[]): void {
    this.allItems = items;
    this.rebuildFzf();
    this.applyFzf(this.userInput);
    this.rerender();
  }

  // ---------------------------------------------------------------------------
  // fzf
  // ---------------------------------------------------------------------------

  private rebuildFzf(): void {
    this.fzfInstance = new Fzf(this.allItems, {
      selector: this.opts.selector ?? ((item: T) => String(item)),
      limit: 50,
      tiebreakers: [byLengthAsc],
      casing: "case-insensitive",
    });
  }

  private applyFzf(query: string): void {
    if (query && this.fzfInstance) {
      this.fzfResults = this.fzfInstance.find(query);
    } else {
      this.fzfResults = wrapAsResults(this.allItems);
    }

    // Clamp cursor
    if (this.fzfResults.length === 0) {
      this.optionsCursor = 0;
      this._setValue(undefined);
    } else {
      this.optionsCursor = Math.min(
        this.optionsCursor,
        this.fzfResults.length - 1,
      );
      this._setValue(this.fzfResults[this.optionsCursor]?.item);
    }
  }

  // ---------------------------------------------------------------------------
  // Spinner
  // ---------------------------------------------------------------------------

  private setLoading(loading: boolean): void {
    this.isLoading = loading;
    if (loading && !this.spinnerInterval) {
      this.spinnerFrame = 0;
      this.spinnerInterval = setInterval(() => {
        this.spinnerFrame =
          (this.spinnerFrame + 1) % SPINNER_FRAMES.length;
        this.rerender();
      }, 80);
    } else if (!loading && this.spinnerInterval) {
      clearInterval(this.spinnerInterval);
      this.spinnerInterval = null;
    }
  }

  // ---------------------------------------------------------------------------
  // Cleanup
  // ---------------------------------------------------------------------------

  private cleanup(): void {
    if (this.spinnerInterval) clearInterval(this.spinnerInterval);
    if (this.debounceTimer) clearTimeout(this.debounceTimer);
    this.abortController?.abort();
    this.spinnerInterval = null;
    this.debounceTimer = null;
  }

  // ---------------------------------------------------------------------------
  // Cursor rendering
  // ---------------------------------------------------------------------------

  get userInputWithCursor(): string {
    if (!this.userInput) return pc.inverse(pc.hidden("_"));
    if (this._cursor >= this.userInput.length)
      return `${this.userInput}\u2588`;
    const before = this.userInput.slice(0, this._cursor);
    const [cursor, ...after] = this.userInput.slice(this._cursor);
    return `${before}${pc.inverse(cursor)}${after.join("")}`;
  }

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  private renderFrame(): string {
    const bar = pc.gray(S_BAR);
    const barEnd = pc.gray(S_BAR_END);

    switch (this.state) {
      case "submit": {
        const selected = this.value;
        const label = selected
          ? this.opts.selector
            ? this.opts.selector(selected).split(" ")[0] // name portion
            : String(selected)
          : "";
        return [
          `${symbol(this.state)}  ${this.opts.message}`,
          `${bar}  ${pc.dim(label)}`,
        ].join("\n");
      }

      case "cancel": {
        const lines = [`${symbol(this.state)}  ${this.opts.message}`];
        if (this.userInput) {
          lines.push(
            `${bar}  ${pc.strikethrough(pc.dim(this.userInput))}`,
          );
        }
        lines.push(bar);
        return lines.join("\n");
      }

      default: {
        // active / initial / error
        const barColor = this.state === "error" ? pc.yellow : pc.cyan;
        const cBar = barColor(S_BAR);
        const cBarEnd = barColor(S_BAR_END);

        // Header
        const headerSymbol = this.isLoading
          ? pc.magenta(SPINNER_FRAMES[this.spinnerFrame])
          : symbol(this.state);
        const lines: string[] = [
          `${headerSymbol}  ${this.opts.message}`,
          cBar,
        ];

        // Search input
        const placeholder = this.opts.placeholder;
        const showPlaceholder = !this.userInput && placeholder;
        const inputDisplay = showPlaceholder
          ? ` ${pc.dim(placeholder)}`
          : this.userInput
            ? ` ${this.userInputWithCursor}`
            : ` ${this.userInputWithCursor}`;
        const matchCount =
          this.fzfResults.length !== this.allItems.length &&
          this.allItems.length > 0
            ? pc.dim(
                ` (${this.fzfResults.length} match${this.fzfResults.length === 1 ? "" : "es"})`,
              )
            : "";
        lines.push(`${cBar}  ${pc.dim("Search:")}${inputDisplay}${matchCount}`);

        // Error
        if (this.state === "error" && this.error) {
          lines.push(`${cBar}  ${pc.yellow(this.error)}`);
        }

        // Options or loading/empty state
        if (this.fzfResults.length === 0 && this.userInput) {
          // Show "No matches found" when local fzf filter has 0 results,
          // even if an async source refetch is still in flight.
          if (this.isLoading) {
            lines.push(`${cBar}  ${pc.yellow("No matches found")}  ${pc.dim("(searching…)")}`);
          } else {
            lines.push(`${cBar}  ${pc.yellow("No matches found")}`);
          }
        } else if (this.isLoading && this.fzfResults.length === 0) {
          lines.push(`${cBar}  ${pc.dim("Searching…")}`);
        } else if (this.fzfResults.length > 0) {
          // Help + footer lines (for rowPadding calculation)
          const helpParts = [
            `${pc.dim("↑/↓")} to select`,
            `${pc.dim("Enter:")} install`,
            `${pc.dim("Type:")} to search`,
          ];
          const footerLines = [
            `${cBar}  ${helpParts.join(" \u2022 ")}`,
            cBarEnd,
          ];

          const optionLines = limitOptions({
            cursor: this.optionsCursor,
            options: this.fzfResults,
            maxItems: this.opts.maxItems,
            columnPadding: 3, // bar + 2 spaces
            rowPadding: lines.length + footerLines.length,
            style: (fzfResult: FzfResultItem<T>, active: boolean) =>
              this.opts.renderItem(
                fzfResult.item,
                active,
                fzfResult.positions.size > 0
                  ? fzfResult.positions
                  : undefined,
              ),
          });

          for (const line of optionLines) {
            lines.push(`${cBar}  ${line}`);
          }

          lines.push(...footerLines);
          return lines.join("\n");
        }

        // Footer (for loading/empty states)
        lines.push(cBarEnd);
        return lines.join("\n");
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export function searchPrompt<T>(
  opts: SearchPromptOptions<T>,
): Promise<T | symbol> {
  return new SearchPromptImpl(opts).prompt() as Promise<T | symbol>;
}
