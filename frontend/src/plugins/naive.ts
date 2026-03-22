import type { App } from 'vue'
import {
  NConfigProvider,
  NMessageProvider,
  NDialogProvider,
  NNotificationProvider,
  darkTheme,
} from 'naive-ui'

export const naiveUIPlugin = {
  install(app: App, options?: { dark?: boolean }) {
    app.component('NConfigProvider', NConfigProvider)
    app.component('NMessageProvider', NMessageProvider)
    app.component('NDialogProvider', NDialogProvider)
    app.component('NNotificationProvider', NNotificationProvider)

    app.provide('naive-theme', options?.dark ? darkTheme : null)
    app.provide('naive-theme-overrides', {
      common: {
        primaryColor: '#6366f1',
        primaryColorHover: '#818cf8',
        primaryColorPressed: '#4f46e5',
      },
    })
  },
}
