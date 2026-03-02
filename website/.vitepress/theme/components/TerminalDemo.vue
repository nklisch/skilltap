<script setup>
import { ref, onMounted } from "vue";

const lines = [
  { type: "cmd",    text: "$ skilltap install user/code-reviewer --global --semantic --also claude-code" },
  { type: "out",    text: "◆  Cloning user/code-reviewer..." },
  { type: "out",    text: "◆  Scanning for skills..." },
  { type: "out",    text: "│" },
  { type: "out",    text: "│  Found 1 skill:" },
  { type: "skill",  text: "│  ● code-reviewer — Review code for bugs and style" },
  { type: "out",    text: "│" },
  { type: "out",    text: "◇  Static scan: 0 warnings" },
  { type: "out",    text: "│" },
  { type: "out",    text: "◆  Semantic scan (4 chunks)..." },
  { type: "check",  text: "│  ✓  chunk 1/4" },
  { type: "check",  text: "│  ✓  chunk 2/4" },
  { type: "check",  text: "│  ✓  chunk 3/4" },
  { type: "check",  text: "│  ✓  chunk 4/4" },
  { type: "out",    text: "│" },
  { type: "out",    text: "◇  Semantic scan: clean" },
  { type: "out",    text: "│" },
  { type: "prompt", text: "◇  Install code-reviewer?" },
  { type: "answer", text: "│  › Yes" },
  { type: "out",    text: "│" },
  { type: "success", text: "◆  Installed code-reviewer" },
  { type: "path",   text: "   → ~/.agents/skills/code-reviewer" },
  { type: "path",   text: "   → ~/.claude/skills/code-reviewer" },
];

function getDelay(line) {
  if (line.type === "cmd") return 800;
  if (line.type === "prompt") return 600;
  if (line.type === "answer") return 900;
  if (line.type === "check") return 280;
  return 150;
}

const visibleLines = ref(0);
const cursorVisible = ref(true);
const typingDone = ref(false);

onMounted(() => {
  let i = 0;
  const show = () => {
    if (i < lines.length) {
      visibleLines.value = ++i;
      setTimeout(show, getDelay(lines[i - 1]));
    } else {
      typingDone.value = true;
    }
  };
  setTimeout(show, 600);

  setInterval(() => {
    cursorVisible.value = !cursorVisible.value;
  }, 530);
});
</script>

<template>
  <div class="terminal">
    <div class="terminal-bar">
      <span class="dot red"></span>
      <span class="dot yellow"></span>
      <span class="dot green"></span>
      <span class="terminal-title">skilltap</span>
    </div>
    <div class="terminal-body">
      <div
        v-for="(line, idx) in lines.slice(0, visibleLines)"
        :key="idx"
        class="line"
        :class="line.type"
      >
        {{ line.text }}
      </div>
      <div v-if="typingDone" class="line cmd">
        $ <span class="cursor" :class="{ hidden: !cursorVisible }">█</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.terminal {
  background: #1c1917;
  border-radius: 12px;
  overflow: hidden;
  border: 1px solid #292524;
  font-family: var(--vp-font-family-mono);
  font-size: 13px;
  line-height: 1.6;
  width: 100%;
  max-width: 560px;
  box-shadow:
    0 0 0 1px rgba(245, 158, 11, 0.06),
    0 25px 50px -12px rgba(0, 0, 0, 0.5);
}

.terminal-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 10px 14px;
  background: #292524;
  border-bottom: 1px solid #292524;
}

.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
}
.dot.red { background: #ef4444; }
.dot.yellow { background: #f59e0b; }
.dot.green { background: #22c55e; }

.terminal-title {
  margin-left: 8px;
  color: #a8a29e;
  font-size: 12px;
}

.terminal-body {
  padding: 16px;
}

.line {
  white-space: pre;
  color: #d6d3d1;
}

.line.cmd {
  color: #f5f5f4;
  font-weight: 500;
}

.line.success {
  color: #4ade80;
  font-weight: 500;
}

.line.skill {
  color: #fbbf24;
}

.line.path {
  color: #a8a29e;
}

.line.prompt {
  color: #e7e5e4;
}

.line.answer {
  color: #fbbf24;
}

.line.check {
  color: #4ade80;
}

.cursor {
  color: #f59e0b;
  animation: none;
}
.cursor.hidden {
  opacity: 0;
}
</style>
