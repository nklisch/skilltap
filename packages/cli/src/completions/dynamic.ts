import {
  loadConfig,
  loadSkillState,
  loadPlugins,
  loadTaps,
} from "@skilltap/core";
import { tryFindProjectRoot } from "../ui/resolve";

async function loadAllSkills() {
  const globalResult = await loadSkillState();
  const projectRoot = await tryFindProjectRoot();
  const projectResult = projectRoot ? await loadSkillState(projectRoot) : null;
  return [
    ...(globalResult.ok ? globalResult.value.skills : []),
    ...(projectResult?.ok ? projectResult.value.skills : []),
  ];
}

async function loadAllPlugins() {
  const globalResult = await loadPlugins();
  const projectRoot = await tryFindProjectRoot();
  const projectResult = projectRoot ? await loadPlugins(projectRoot) : null;
  return [
    ...(globalResult.ok ? globalResult.value.plugins : []),
    ...(projectResult?.ok ? projectResult.value.plugins : []),
  ];
}

export async function printCompletions(type: string): Promise<void> {
  switch (type) {
    case "installed-skills": {
      const skills = await loadAllSkills();
      for (const s of skills) console.log(s.name);
      break;
    }
    case "linked-skills": {
      const skills = await loadAllSkills();
      for (const s of skills) {
        if (s.scope === "linked") console.log(s.name);
      }
      break;
    }
    case "active-skills": {
      const skills = await loadAllSkills();
      for (const s of skills) {
        if (s.active !== false) console.log(s.name);
      }
      break;
    }
    case "disabled-skills": {
      const skills = await loadAllSkills();
      for (const s of skills) {
        if (s.active === false) console.log(s.name);
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
    case "installed-plugins": {
      const plugins = await loadAllPlugins();
      for (const p of plugins) console.log(p.name);
      break;
    }
  }
}
