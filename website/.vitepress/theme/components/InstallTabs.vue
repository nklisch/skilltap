<script setup lang="ts">
import { ref } from "vue";

const active = ref("curl");
const tabs = [
  { id: "curl", label: "curl" },
  { id: "bunx", label: "bunx" },
  { id: "npx", label: "npx" },
  { id: "binary", label: "Binary" },
];
const commands: Record<string, string> = {
  curl: "curl -fsSL https://skilltap.dev/install.sh | sh",
  bunx: "bunx skilltap --help",
  npx: "npx skilltap --help",
  binary: "# Download from GitHub Releases\nhttps://github.com/nklisch/skilltap/releases",
};

const copied = ref(false);
function copy() {
  navigator.clipboard.writeText(commands[active.value]);
  copied.value = true;
  setTimeout(() => (copied.value = false), 2000);
}
</script>

<template>
  <div class="install-tabs">
    <div class="tabs-header">
      <button
        v-for="tab in tabs"
        :key="tab.id"
        class="tab"
        :class="{ active: active === tab.id }"
        @click="active = tab.id"
      >
        {{ tab.label }}
      </button>
      <button class="copy-btn" @click="copy" :title="copied ? 'Copied!' : 'Copy'">
        {{ copied ? '✓' : '⎘' }}
      </button>
    </div>
    <pre class="install-code"><code>{{ commands[active] }}</code></pre>
  </div>
</template>

<style scoped>
.install-tabs {
  background: #1c1917;
  border: 1px solid #292524;
  border-radius: 12px;
  overflow: hidden;
  max-width: 600px;
  width: 100%;
}

.tabs-header {
  display: flex;
  align-items: center;
  gap: 0;
  border-bottom: 1px solid #292524;
  padding: 0 4px;
}

.tab {
  background: none;
  border: none;
  color: #a8a29e;
  font-family: var(--vp-font-family-mono);
  font-size: 13px;
  padding: 10px 16px;
  cursor: pointer;
  border-bottom: 2px solid transparent;
  transition: color 0.2s, border-color 0.2s;
}

.tab:hover {
  color: #f5f5f4;
}

.tab.active {
  color: #fbbf24;
  border-bottom-color: #f59e0b;
}

.copy-btn {
  margin-left: auto;
  background: none;
  border: none;
  color: #a8a29e;
  font-size: 16px;
  padding: 8px 12px;
  cursor: pointer;
  transition: color 0.2s;
}

.copy-btn:hover {
  color: #f5f5f4;
}

.install-code {
  margin: 0;
  padding: 16px 20px;
  font-family: var(--vp-font-family-mono);
  font-size: 14px;
  line-height: 1.6;
  color: #f5f5f4;
  overflow-x: auto;
  background: transparent;
}

.install-code code {
  background: none;
  padding: 0;
}
</style>
