<template>
  <div class="flow-diagram">
    <div class="flow-row sources">
      <div class="flow-node source">
        <div class="node-icon">GitHub</div>
      </div>
      <div class="flow-node source">
        <div class="node-icon">GitLab</div>
      </div>
      <div class="flow-node source">
        <div class="node-icon">Gitea</div>
      </div>
    </div>

    <div class="flow-arrow down">
      <svg width="24" height="40" viewBox="0 0 24 40">
        <path d="M12 0 L12 32 M6 26 L12 34 L18 26" stroke="#f59e0b" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
      <span class="arrow-label">git clone</span>
    </div>

    <div class="flow-row center">
      <div class="flow-node skilltap">
        <div class="node-label">skilltap</div>
        <div class="node-sub">scan &amp; install</div>
      </div>
    </div>

    <div class="flow-arrow down">
      <svg width="24" height="40" viewBox="0 0 24 40">
        <path d="M12 0 L12 32 M6 26 L12 34 L18 26" stroke="#f59e0b" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
      <span class="arrow-label">symlink</span>
    </div>

    <div class="flow-row agents">
      <div class="flow-node agent">
        <div class="node-path">~/.agents/skills/</div>
      </div>
    </div>

    <div class="flow-fanout">
      <div class="fanout-item">
        <span class="fanout-arrow">→</span>
        <span class="fanout-path">.claude/skills/</span>
      </div>
      <div class="fanout-item">
        <span class="fanout-arrow">→</span>
        <span class="fanout-path">.cursor/skills/</span>
      </div>
      <div class="fanout-item">
        <span class="fanout-arrow">→</span>
        <span class="fanout-path">.codex/skills/</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.flow-diagram {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0;
  padding: 20px 0;
}

.flow-row {
  display: flex;
  gap: 16px;
  justify-content: center;
  flex-wrap: wrap;
}

.flow-node {
  background: #1c1917;
  border: 1px solid #292524;
  border-radius: 10px;
  padding: 12px 20px;
  text-align: center;
  font-family: var(--vp-font-family-mono);
  font-size: 13px;
}

.flow-node.source {
  color: #d6d3d1;
}

.flow-node.skilltap {
  border-color: #f59e0b;
  background: rgba(245, 158, 11, 0.08);
  padding: 16px 32px;
}

.node-label {
  color: #fbbf24;
  font-weight: 600;
  font-size: 16px;
}

.node-sub {
  color: #a8a29e;
  font-size: 12px;
  margin-top: 2px;
}

.flow-node.agent {
  border-color: #4ade80;
  background: rgba(74, 222, 128, 0.06);
}

.node-path {
  color: #4ade80;
  font-size: 13px;
}

.flow-arrow {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 4px 0;
}

.arrow-label {
  font-family: var(--vp-font-family-mono);
  font-size: 11px;
  color: #78716c;
}

.fanout-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-family: var(--vp-font-family-mono);
  font-size: 13px;
  padding: 4px 0;
}

.fanout-arrow {
  color: #f59e0b;
}

.fanout-path {
  color: #a8a29e;
}

.fanout {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 2px;
  margin-top: 8px;
}
</style>
