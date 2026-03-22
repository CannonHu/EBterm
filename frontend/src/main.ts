import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { naiveUIPlugin } from './plugins/naive'
import router from './router'
import App from './App.vue'

const app = createApp(App)

app.use(createPinia())
app.use(naiveUIPlugin)
app.use(router)

app.mount('#app')
