import {createRouter, createWebHistory} from 'vue-router'
import List from './List.vue'

// Lazy load
const Read = () => import('./Read.vue');

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    { path: '/', redirect: '/list' },
    { path: '/list', component: List, props: route => ({ path: route.query.path }) },
    { path: '/read', component: Read, props: route => ({ path: route.query.path }) },
  ],
  scrollBehavior: async function (to, from, savedPosition): Promise<any> {
    if (savedPosition) {
      return savedPosition
    } else {
      return { x: 0, y: 0 }
    }
  },
})

export default router
