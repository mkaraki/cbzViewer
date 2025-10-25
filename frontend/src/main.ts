import {createApp} from 'vue'
import App from './App.vue'
import router from './router'
import * as Sentry from "@sentry/vue";

const app = createApp(App)

Sentry.init({
    app,
    dsn: document.body.dataset.sentryDsn ?? undefined,
    sendDefaultPii: false,
    integrations: [
        Sentry.browserTracingIntegration({ router }),
        Sentry.replayIntegration(),
        Sentry.consoleLoggingIntegration(),
    ],
    enableLogs: true,
    tracesSampleRate: 1.0,
    tracePropagationTargets: document.body.dataset.serverHost ? [document.body.dataset.serverHost] : undefined,
    replaysSessionSampleRate: 0.1,
    replaysOnErrorSampleRate: 1.0,
});

app.use(router)

app.mount('#app')
