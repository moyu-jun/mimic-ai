<script setup lang="ts">
/**
 * 阶段 1：仅渲染一个深色圆角的空容器，验证窗口外观。
 * 后续阶段会在 .app-container 内填充标题栏、菜单、内容区、状态栏。
 */
</script>

<template>
  <div class="app-container"></div>
</template>

<style scoped>
.app-container {
  width: var(--window-width);
  height: var(--window-height);
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: var(--window-radius);
  overflow: hidden;
  /* 进出场动画占位，子组件 hover 等可复用 var(--transition-fast) */
  transition: background var(--transition-normal) var(--ease-default);
}
</style>
