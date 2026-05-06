export * from "./basic";
export * from "./multi";
export * from "./npm";

export const TEMPLATE_NAMES = ["basic", "npm", "multi"] as const;
export type TemplateName = (typeof TEMPLATE_NAMES)[number];
