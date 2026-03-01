import type { AgentResponse } from "../schemas/agent";
import type { Result, ScanError } from "../types";

export interface AgentAdapter {
  readonly name: string;
  readonly cliName: string;
  detect(): Promise<boolean>;
  invoke(prompt: string): Promise<Result<AgentResponse, ScanError>>;
}
