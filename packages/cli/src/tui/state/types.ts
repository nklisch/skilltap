export type Screen = "dashboard" | "find" | "toggle" | "adopt";

export type DashboardTab = "installed" | "taps" | "updates" | "drift";

export interface DashboardState {
  tab: DashboardTab;
  selectedIndex: number;
  loading: boolean;
}

export interface FindState {
  query: string;
  results: FindResult[];
  selectedIndex: number;
  loading: boolean;
}

export interface FindResult {
  name: string;
  description: string;
  source: string;
  type: "skill" | "plugin";
}

export interface ToggleState {
  step: "type" | "name" | "components";
  type: "skill" | "plugin" | "mcp" | null;
  selectedName: string | null;
  components: { name: string; active: boolean }[];
  selectedComponentIndices: number[];
}

export interface AdoptState {
  candidates: AdoptCandidate[];
  focusIndex: number;
  selectedIndices: number[];
  perItemMode: Map<string, "track-in-place" | "move">;
  loading: boolean;
}

export interface AdoptCandidate {
  kind: "skill" | "plugin";
  name: string;
  source: string;
  description?: string;
}

export type AppState =
  | { screen: "dashboard"; state: DashboardState }
  | { screen: "find"; state: FindState }
  | { screen: "toggle"; state: ToggleState }
  | { screen: "adopt"; state: AdoptState };

export type Action =
  | { type: "navigate"; screen: Screen }
  | { type: "exit" }
  | { type: "dashboard:tab"; tab: DashboardTab }
  | { type: "dashboard:cursor"; delta: -1 | 1 }
  | { type: "find:query"; query: string }
  | { type: "find:results"; results: FindResult[] }
  | { type: "find:cursor"; delta: -1 | 1 }
  | { type: "toggle:step-back" }
  | { type: "toggle:set-type"; value: ToggleState["type"] }
  | { type: "toggle:set-name"; value: string }
  | { type: "toggle:components-loaded"; components: ToggleState["components"] }
  | { type: "toggle:component-toggle"; index: number }
  | { type: "adopt:candidates-loaded"; candidates: AdoptCandidate[] }
  | { type: "adopt:cursor"; delta: -1 | 1 }
  | { type: "adopt:select-toggle" }
  | { type: "adopt:mode-toggle" };

export interface AppContext {
  dispatchInstall: (
    type: "skill" | "plugin" | "mcp",
    source: string,
  ) => Promise<{ ok: boolean; error?: string }>;
  dispatchToggle: (
    type: "skill" | "plugin" | "mcp",
    name: string,
    component?: string,
  ) => Promise<{ ok: boolean; error?: string }>;
  dispatchAdopt: (
    kind: "skill" | "plugin",
    name: string,
    mode: "track-in-place" | "move",
  ) => Promise<{ ok: boolean; error?: string }>;
  dispatchSync: () => Promise<{ ok: boolean; error?: string }>;
  loadDashboardData: (tab: DashboardTab) => Promise<unknown>;
  loadFindResults: (query: string) => Promise<FindResult[]>;
  loadToggleComponents: (
    type: "skill" | "plugin" | "mcp",
    name: string,
  ) => Promise<ToggleState["components"]>;
  loadAdoptCandidates: () => Promise<AdoptCandidate[]>;
}
