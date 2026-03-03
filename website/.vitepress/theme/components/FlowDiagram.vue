<script setup>
const devs = [
  { name: "Dev 1", agent: "Claude Code" },
  { name: "Dev 2", agent: "Cursor" },
  { name: "Dev 3", agent: "Codex" },
  { name: "Dev 4", agent: "Gemini" },
];

const hosts = ["GitHub", "GitLab", "Gitea", "your server"];
</script>

<template>
  <div class="flow-diagram">

    <!-- Left: Publish phase -->
    <div class="phase left-phase">
      <div class="phase-label">Publish once</div>

      <div class="flow-node author-node">
        <div class="node-title">Your team</div>
        <div class="node-sub">maintains SKILL.md files</div>
      </div>

      <div class="v-connector">
        <div class="v-line"></div>
        <svg class="v-arrowhead" width="14" height="8" viewBox="0 0 14 8">
          <path d="M1 1 L7 7 L13 1" stroke="#f59e0b" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        <span class="connector-label">git push</span>
      </div>

      <div class="flow-node tap-node">
        <div class="node-title">Company tap</div>
        <div class="host-tags">
          <span v-for="h in hosts" :key="h" class="host-tag">{{ h }}</span>
        </div>
      </div>
    </div>

    <!-- Center: skilltap bridge -->
    <div class="bridge">
      <div class="bridge-line"></div>
      <div class="bridge-node">
        <div class="bridge-name">skilltap</div>
        <div class="bridge-ops">
          <span>tap add</span>
          <span>find</span>
          <span>install</span>
          <span>scan</span>
        </div>
      </div>
      <div class="bridge-line"></div>
      <svg class="bridge-arrowhead" width="8" height="14" viewBox="0 0 8 14">
        <path d="M1 1 L7 7 L1 13" stroke="#f59e0b" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </div>

    <!-- Right: Install phase -->
    <div class="phase right-phase">
      <div class="phase-label">Install everywhere</div>

      <div class="dev-list">
        <div v-for="dev in devs" :key="dev.agent" class="dev-row">
          <div class="dev-machine">{{ dev.name }}</div>
          <svg class="dev-arrow" width="28" height="10" viewBox="0 0 28 10">
            <path d="M0 5 L22 5 M16 1 L22 5 L16 9" stroke="#78716c" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          <div class="agent-chip">{{ dev.agent }}</div>
        </div>
      </div>

      <div class="install-path">~/.agents/skills/</div>
    </div>

  </div>
</template>

<style scoped>
.flow-diagram {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  gap: 0;
  align-items: center;
  max-width: 820px;
  margin: 0 auto;
}

/* Phases */
.phase {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0;
}

.phase-label {
  font-family: var(--vp-font-family-mono);
  font-size: 11px;
  font-weight: 600;
  color: #78716c;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  margin-bottom: 16px;
}

/* Nodes */
.flow-node {
  background: #1c1917;
  border: 1px solid #292524;
  border-radius: 10px;
  padding: 14px 24px;
  text-align: center;
  font-family: var(--vp-font-family-mono);
  width: 100%;
  max-width: 220px;
}

.author-node {
  border-color: #44403c;
}

.tap-node {
  border-color: #44403c;
}

.node-title {
  font-size: 14px;
  font-weight: 600;
  color: #d6d3d1;
}

.node-sub {
  font-size: 11px;
  color: #78716c;
  margin-top: 4px;
}

.host-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  justify-content: center;
  margin-top: 8px;
}

.host-tag {
  font-size: 10px;
  color: #a8a29e;
  background: #292524;
  border-radius: 4px;
  padding: 2px 6px;
}

/* Vertical connector inside left phase */
.v-connector {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 6px 0;
  position: relative;
}

.v-line {
  width: 1px;
  height: 24px;
  background: #44403c;
}

.v-arrowhead {
  display: block;
}

.connector-label {
  font-family: var(--vp-font-family-mono);
  font-size: 10px;
  color: #57534e;
  position: absolute;
  left: calc(50% + 10px);
  top: 50%;
  transform: translateY(-50%);
  white-space: nowrap;
}

/* Bridge */
.bridge {
  display: flex;
  align-items: center;
  padding: 0 8px;
}

.bridge-line {
  height: 1px;
  width: 28px;
  background: #44403c;
}

.bridge-node {
  background: rgba(245, 158, 11, 0.07);
  border: 1px solid rgba(245, 158, 11, 0.35);
  border-radius: 10px;
  padding: 14px 20px;
  text-align: center;
  font-family: var(--vp-font-family-mono);
}

.bridge-name {
  font-size: 15px;
  font-weight: 700;
  color: #fbbf24;
}

.bridge-ops {
  display: flex;
  gap: 6px;
  justify-content: center;
  flex-wrap: wrap;
  margin-top: 8px;
}

.bridge-ops span {
  font-size: 10px;
  color: #a8a29e;
  background: #1c1917;
  border: 1px solid #292524;
  border-radius: 4px;
  padding: 2px 6px;
}

.bridge-arrowhead {
  display: block;
  flex-shrink: 0;
}

/* Dev list on the right */
.right-phase {
  align-items: flex-start;
  padding-left: 8px;
}

.dev-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  width: 100%;
}

.dev-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.dev-machine {
  font-family: var(--vp-font-family-mono);
  font-size: 12px;
  color: #78716c;
  width: 40px;
  flex-shrink: 0;
}

.dev-arrow {
  flex-shrink: 0;
}

.agent-chip {
  font-family: var(--vp-font-family-mono);
  font-size: 12px;
  color: #a8a29e;
  background: #1c1917;
  border: 1px solid #292524;
  border-radius: 6px;
  padding: 4px 10px;
}

.install-path {
  font-family: var(--vp-font-family-mono);
  font-size: 11px;
  color: #4ade80;
  margin-top: 14px;
  padding: 6px 10px;
  background: rgba(74, 222, 128, 0.05);
  border: 1px solid rgba(74, 222, 128, 0.15);
  border-radius: 6px;
  align-self: center;
}

/* Responsive: stack vertically on mobile */
@media (max-width: 640px) {
  .flow-diagram {
    grid-template-columns: 1fr;
    gap: 24px;
  }

  .bridge {
    flex-direction: column;
    padding: 0;
  }

  .bridge-line {
    width: 1px;
    height: 20px;
  }

  .bridge-arrowhead {
    transform: rotate(90deg);
  }

  .right-phase {
    align-items: center;
    padding-left: 0;
  }

  .connector-label {
    display: none;
  }
}
</style>
