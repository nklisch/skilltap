export * from "./basic";
export * from "./npm";
export * from "./multi";

export const TEMPLATE_NAMES = ["basic", "npm", "multi"] as const;
export type TemplateName = (typeof TEMPLATE_NAMES)[number];
