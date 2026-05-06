import type { PluginRecord, StoredComponent } from "../schemas/plugins";

export interface ParsedComponentRef {
  name: string;
  /** Component name after the colon, or undefined if no colon was present. */
  component?: string;
}

// Parse "foo:bar" → { name: "foo", component: "bar" }; "foo" → { name: "foo" }.
// First colon splits; remainder is the component name (component names may
// contain colons, e.g. namespaced MCP server names).
//
// Malformed inputs ":bar" (empty plugin name) and "foo:" (empty component)
// fall back to name-only — the caller surfaces a clearer error than parsing
// can express on its own.
export function parseComponentRef(input: string): ParsedComponentRef {
  const colonIdx = input.indexOf(":");
  if (colonIdx === -1) return { name: input };
  if (colonIdx === 0 || colonIdx === input.length - 1) {
    return { name: input };
  }
  return {
    name: input.slice(0, colonIdx),
    component: input.slice(colonIdx + 1),
  };
}

// Look up a component by name within a plugin record. Returns null if not found.
// When multiple components share a name across types, returns the first match
// in iteration order (manifest order).
export function findComponentInPlugin(
  plugin: PluginRecord,
  componentName: string,
): StoredComponent | null {
  return plugin.components.find((c) => c.name === componentName) ?? null;
}
