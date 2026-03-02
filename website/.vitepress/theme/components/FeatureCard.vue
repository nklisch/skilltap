<script setup lang="ts">
defineProps<{
  icon: string;
  title: string;
  description: string;
}>();
</script>

<template>
  <div class="feature-card">
    <div class="feature-icon">{{ icon }}</div>
    <h3 class="feature-title">{{ title }}</h3>
    <p class="feature-desc">{{ description }}</p>
  </div>
</template>

<style scoped>
.feature-card {
  background: #1c1917;
  border: 1px solid #292524;
  border-radius: 12px;
  padding: 28px 24px;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.feature-card:hover {
  border-color: rgba(245, 158, 11, 0.3);
  box-shadow: 0 0 20px rgba(245, 158, 11, 0.05);
}

.feature-icon {
  font-size: 28px;
  margin-bottom: 12px;
}

.feature-title {
  font-family: var(--vp-font-family-mono);
  font-size: 16px;
  font-weight: 600;
  color: #f5f5f4;
  margin: 0 0 8px;
}

.feature-desc {
  font-size: 14px;
  line-height: 1.6;
  color: #a8a29e;
  margin: 0;
}
</style>
