/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}

declare module "naive-ui" {
  export const NConfigProvider: any;
  export const NMessageProvider: any;
  export const NDialogProvider: any;
  export const NNotificationProvider: any;
  export const darkTheme: any;
  export const NGlobalStyle: any;
  export const NLayout: any;
  export const NLayoutSider: any;
  export const NLayoutHeader: any;
  export const NLayoutContent: any;
  export type GlobalThemeOverrides = any;
}