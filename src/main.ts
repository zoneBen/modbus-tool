import "@fontsource-variable/geist";
import "@fontsource-variable/jetbrains-mono";
import { createPinia } from "pinia";
import { createApp } from "vue";
import App from "./App.vue";
import { i18n } from "./i18n";
import "./styles/tailwind.css";

const app = createApp(App);
app.use(createPinia());
app.use(i18n);
app.mount("#app");
