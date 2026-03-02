import { generateBashCompletions } from "./bash";
import { generateFishCompletions } from "./fish";
import { generateZshCompletions } from "./zsh";

export type Shell = "bash" | "zsh" | "fish";

export function generateCompletions(shell: Shell): string {
  switch (shell) {
    case "bash":
      return generateBashCompletions();
    case "zsh":
      return generateZshCompletions();
    case "fish":
      return generateFishCompletions();
  }
}
