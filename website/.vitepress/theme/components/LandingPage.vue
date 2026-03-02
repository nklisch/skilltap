<script setup>
import { useData } from "vitepress";
import TerminalDemo from "./TerminalDemo.vue";
import SecurityScanDemo from "./SecurityScanDemo.vue";
import FeatureCard from "./FeatureCard.vue";
import FlowDiagram from "./FlowDiagram.vue";
import InstallTabs from "./InstallTabs.vue";

const { site } = useData();

const features = [
  {
    icon: "⚡",
    title: "Git-native",
    description:
      "Clone from Gitea, GitLab, GitHub, Forgejo — any git host. Your SSH keys and credential helpers just work.",
  },
  {
    icon: "🔌",
    title: "Agent-agnostic",
    description:
      "Installs to .agents/skills/. Symlinks to Claude, Cursor, Codex, Gemini, Windsurf automatically.",
  },
  {
    icon: "🍺",
    title: "Taps",
    description:
      "Curated skill indexes from any git repo. Add a friend's tap, your company's tap, the community tap. Search across all of them.",
  },
  {
    icon: "🛡️",
    title: "Security scanning",
    description:
      "Static analysis on every install. Optional LLM-powered semantic scan using your own agent. Nothing hidden.",
  },
  {
    icon: "📊",
    title: "Diff-aware updates",
    description:
      "skilltap update fetches, diffs, re-scans changed lines, then applies. You see what changed before it lands.",
  },
  {
    icon: "🤖",
    title: "Agent mode",
    description:
      "Safe headless operation for AI agents. Security issues block with machine-readable stop directives.",
  },
];
</script>

<template>
  <div class="landing">
    <!-- Nav -->
    <nav class="landing-nav">
      <div class="nav-inner">
        <a href="/" class="nav-logo">
          <span class="logo-text">skilltap</span>
        </a>
        <div class="nav-links">
          <a href="/guide/what-is-skilltap">Guide</a>
          <a href="/reference/cli">Reference</a>
          <a href="https://github.com/nklisch/skilltap" target="_blank" rel="noopener">GitHub</a>
        </div>
      </div>
    </nav>

    <!-- Hero -->
    <section class="hero">
      <div class="hero-inner">
        <div class="hero-content">
          <h1 class="hero-title">
            <span class="title-main">skilltap</span>
          </h1>
          <p class="hero-tagline">Agent skills, on tap.</p>
          <p class="hero-subtitle">
            Install SKILL.md files from any git host. Works with every agent.
          </p>
          <div class="hero-actions">
            <a href="/guide/getting-started" class="btn btn-primary">Get Started</a>
            <a
              href="https://github.com/nklisch/skilltap"
              target="_blank"
              rel="noopener"
              class="btn btn-outline"
            >
              GitHub
            </a>
          </div>
        </div>
        <div class="hero-demo">
          <TerminalDemo />
        </div>
      </div>
    </section>

    <!-- Install -->
    <section class="install-section">
      <div class="section-inner">
        <InstallTabs />
      </div>
    </section>

    <!-- Features -->
    <section class="features-section">
      <div class="section-inner">
        <h2 class="section-title">Why skilltap?</h2>
        <div class="features-grid">
          <FeatureCard
            v-for="f in features"
            :key="f.title"
            :icon="f.icon"
            :title="f.title"
            :description="f.description"
          />
        </div>
      </div>
    </section>

    <!-- How it works -->
    <section class="how-section">
      <div class="section-inner">
        <h2 class="section-title">How it works</h2>
        <p class="section-subtitle">
          Skills flow from any source through security scanning to your agent's skill directory.
        </p>
        <FlowDiagram />
      </div>
    </section>

    <!-- Quick demo -->
    <section class="demo-section">
      <div class="section-inner">
        <h2 class="section-title">Five minutes to your first skill</h2>
        <div class="demo-code">
          <div class="code-window">
            <div class="code-bar">
              <span class="dot red"></span>
              <span class="dot yellow"></span>
              <span class="dot green"></span>
            </div>
            <pre class="code-body"><code><span class="c-comment"># Add a tap (registry of skills)</span>
<span class="c-prompt">$</span> skilltap tap add community https://github.com/example/skills-tap

<span class="c-comment"># Search for skills</span>
<span class="c-prompt">$</span> skilltap find review
<span class="c-dim">  community/code-reviewer  Review code for bugs and style issues</span>
<span class="c-dim">  community/pr-review      Generate PR review comments</span>

<span class="c-comment"># Install one</span>
<span class="c-prompt">$</span> skilltap install code-reviewer --global --also claude-code
<span class="c-success">◆  Installed code-reviewer</span>

<span class="c-comment"># That's it. Your agent can use it now.</span></code></pre>
          </div>
        </div>
      </div>
    </section>

    <!-- Security in action -->
    <section class="security-section">
      <div class="section-inner">
        <h2 class="section-title">Security in action</h2>
        <p class="section-subtitle">
          Static patterns catch hidden Unicode, obfuscated URLs, and injection attempts. Semantic scan sends each chunk to your local agent for a second opinion.
        </p>
        <div class="security-demo">
          <SecurityScanDemo />
        </div>
        <p class="security-note">
          With <code>--strict</code>, any warning aborts immediately. In agent mode, security issues emit a machine-readable stop directive.
          <a href="/guide/security">Learn more →</a>
        </p>
      </div>
    </section>

    <!-- CTA -->
    <section class="cta-section">
      <div class="section-inner">
        <h2 class="cta-title">Get started in 5 minutes</h2>
        <p class="cta-subtitle">
          Install skilltap, add a tap, and install your first skill.
        </p>
        <div class="cta-actions">
          <a href="/guide/getting-started" class="btn btn-primary btn-lg">
            Read the guide
          </a>
        </div>
      </div>
    </section>

    <!-- Footer -->
    <footer class="landing-footer">
      <div class="footer-inner">
        <span class="footer-text">skilltap — MIT License</span>
        <a
          href="https://github.com/nklisch/skilltap"
          target="_blank"
          rel="noopener"
          class="footer-link"
        >
          GitHub
        </a>
      </div>
    </footer>
  </div>
</template>

<style scoped>
.landing {
  background: #0c0a09;
  color: #f5f5f4;
  min-height: 100vh;
}

/* Nav */
.landing-nav {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  z-index: 100;
  background: rgba(12, 10, 9, 0.85);
  backdrop-filter: blur(12px);
  border-bottom: 1px solid #292524;
}

.nav-inner {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 24px;
  height: 64px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.nav-logo {
  text-decoration: none;
}

.logo-text {
  font-family: var(--vp-font-family-mono);
  font-size: 20px;
  font-weight: 700;
  color: #fbbf24;
}

.nav-links {
  display: flex;
  gap: 24px;
}

.nav-links a {
  font-family: var(--vp-font-family-mono);
  font-size: 14px;
  color: #a8a29e;
  text-decoration: none;
  transition: color 0.2s;
}

.nav-links a:hover {
  color: #f5f5f4;
}

/* Hero */
.hero {
  padding: 140px 24px 80px;
}

.hero-inner {
  max-width: 1200px;
  margin: 0 auto;
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 60px;
  align-items: center;
}

.hero-title {
  margin: 0 0 16px;
}

.title-main {
  font-family: var(--vp-font-family-mono);
  font-size: 64px;
  font-weight: 700;
  background: linear-gradient(135deg, #fbbf24 0%, #f59e0b 50%, #d97706 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.hero-tagline {
  font-family: var(--vp-font-family-mono);
  font-size: 28px;
  font-weight: 500;
  color: #f5f5f4;
  margin: 0 0 12px;
}

.hero-subtitle {
  font-size: 18px;
  color: #a8a29e;
  margin: 0 0 32px;
  line-height: 1.6;
}

.hero-actions {
  display: flex;
  gap: 12px;
}

.hero-demo {
  display: flex;
  justify-content: flex-end;
}

/* Buttons */
.btn {
  display: inline-flex;
  align-items: center;
  padding: 10px 24px;
  border-radius: 8px;
  font-family: var(--vp-font-family-mono);
  font-size: 14px;
  font-weight: 600;
  text-decoration: none;
  transition: all 0.2s;
  cursor: pointer;
}

.btn-primary {
  background: #f59e0b;
  color: #0c0a09;
}

.btn-primary:hover {
  background: #fbbf24;
}

.btn-outline {
  background: transparent;
  color: #d6d3d1;
  border: 1px solid #44403c;
}

.btn-outline:hover {
  border-color: #78716c;
  color: #f5f5f4;
}

.btn-lg {
  padding: 14px 32px;
  font-size: 16px;
}

/* Sections */
.section-inner {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 24px;
}

.section-title {
  font-family: var(--vp-font-family-mono);
  font-size: 32px;
  font-weight: 700;
  color: #f5f5f4;
  text-align: center;
  margin: 0 0 12px;
}

.section-subtitle {
  font-size: 16px;
  color: #a8a29e;
  text-align: center;
  margin: 0 0 40px;
}

/* Install */
.install-section {
  padding: 40px 24px 80px;
  display: flex;
  justify-content: center;
}

/* Features */
.features-section {
  padding: 80px 24px;
  background: #0c0a09;
}

.features-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 20px;
  margin-top: 40px;
}

/* How it works */
.how-section {
  padding: 80px 24px;
  background: rgba(28, 25, 23, 0.5);
}

/* Demo */
.demo-section {
  padding: 80px 24px;
}

.demo-code {
  max-width: 680px;
  margin: 0 auto;
}

.code-window {
  background: #1c1917;
  border-radius: 12px;
  border: 1px solid #292524;
  overflow: hidden;
}

.code-bar {
  display: flex;
  gap: 6px;
  padding: 10px 14px;
  background: #292524;
}

.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
}
.dot.red { background: #ef4444; }
.dot.yellow { background: #f59e0b; }
.dot.green { background: #22c55e; }

.code-body {
  padding: 20px;
  margin: 0;
  font-family: var(--vp-font-family-mono);
  font-size: 13px;
  line-height: 1.8;
  overflow-x: auto;
  background: transparent;
}

.code-body code {
  background: none;
}

.c-comment { color: #78716c; }
.c-prompt { color: #fbbf24; }
.c-dim { color: #a8a29e; }
.c-success { color: #4ade80; }

/* Security in action */
.security-section {
  padding: 80px 24px;
  background: rgba(28, 25, 23, 0.5);
}

.security-demo {
  display: flex;
  justify-content: center;
  margin-top: 40px;
}

.security-note {
  text-align: center;
  font-family: var(--vp-font-family-mono);
  font-size: 13px;
  color: #78716c;
  margin-top: 24px;
}

.security-note code {
  background: #292524;
  padding: 1px 6px;
  border-radius: 4px;
  color: #d6d3d1;
}

.security-note a {
  color: #a8a29e;
  text-decoration: none;
  margin-left: 8px;
}

.security-note a:hover {
  color: #f5f5f4;
}

/* CTA */
.cta-section {
  padding: 100px 24px;
  text-align: center;
  background: linear-gradient(180deg, rgba(245, 158, 11, 0.04) 0%, transparent 100%);
}

.cta-title {
  font-family: var(--vp-font-family-mono);
  font-size: 36px;
  font-weight: 700;
  color: #f5f5f4;
  margin: 0 0 12px;
}

.cta-subtitle {
  font-size: 18px;
  color: #a8a29e;
  margin: 0 0 32px;
}

.cta-actions {
  display: flex;
  justify-content: center;
  gap: 12px;
}

/* Footer */
.landing-footer {
  border-top: 1px solid #292524;
  padding: 24px;
}

.footer-inner {
  max-width: 1200px;
  margin: 0 auto;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.footer-text {
  font-size: 13px;
  color: #78716c;
}

.footer-link {
  font-family: var(--vp-font-family-mono);
  font-size: 13px;
  color: #a8a29e;
  text-decoration: none;
}

.footer-link:hover {
  color: #f5f5f4;
}

/* Responsive */
@media (max-width: 768px) {
  .hero-inner {
    grid-template-columns: 1fr;
    gap: 40px;
  }

  .title-main {
    font-size: 40px;
  }

  .hero-tagline {
    font-size: 22px;
  }

  .hero-demo {
    justify-content: center;
  }

  .features-grid {
    grid-template-columns: 1fr;
  }

  .section-title {
    font-size: 24px;
  }

  .cta-title {
    font-size: 28px;
  }

  .nav-links {
    gap: 16px;
  }
}

@media (max-width: 960px) and (min-width: 769px) {
  .features-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}
</style>
