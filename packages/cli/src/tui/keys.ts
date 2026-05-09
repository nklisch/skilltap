export interface KeyBinding {
  key: string;
  modifier?: "ctrl" | "shift";
  description: string;
  action: { type: string; [k: string]: unknown };
}

export const GLOBAL_KEYS: KeyBinding[] = [
  { key: "q", description: "Quit", action: { type: "exit" } },
  {
    key: "1",
    description: "Dashboard",
    action: { type: "navigate", screen: "dashboard" },
  },
  {
    key: "2",
    description: "Find",
    action: { type: "navigate", screen: "find" },
  },
  {
    key: "3",
    description: "Toggle",
    action: { type: "navigate", screen: "toggle" },
  },
  {
    key: "4",
    description: "Adopt",
    action: { type: "navigate", screen: "adopt" },
  },
];
