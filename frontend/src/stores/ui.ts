import { defineStore } from 'pinia';
import { ref } from 'vue';

export const useUIStore = defineStore('ui', () => {
  const isDark = ref(false);
  const sidebarCollapsed = ref(false);

  function toggleDark() {
    isDark.value = !isDark.value;
  }

  function toggleSidebar() {
    sidebarCollapsed.value = !sidebarCollapsed.value;
  }

  return {
    isDark,
    sidebarCollapsed,
    toggleDark,
    toggleSidebar
  };
});
