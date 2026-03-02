import { loadConfig, loadInstalled, loadTaps } from "@skilltap/core";

export async function printCompletions(type: string): Promise<void> {
  switch (type) {
    case "installed-skills": {
      const installed = await loadInstalled();
      if (installed.ok) {
        for (const s of installed.value.skills) console.log(s.name);
      }
      break;
    }
    case "linked-skills": {
      const installed = await loadInstalled();
      if (installed.ok) {
        for (const s of installed.value.skills) {
          if (s.scope === "linked") console.log(s.name);
        }
      }
      break;
    }
    case "tap-skills": {
      const taps = await loadTaps();
      if (taps.ok) {
        for (const entry of taps.value) console.log(entry.skill.name);
      }
      break;
    }
    case "tap-names": {
      const config = await loadConfig();
      if (config.ok) {
        for (const tap of config.value.taps) console.log(tap.name);
      }
      break;
    }
  }
}
