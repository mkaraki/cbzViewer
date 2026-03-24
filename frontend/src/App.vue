<script lang="ts" setup>
import 'bootstrap/scss/bootstrap.scss';
import './style/global.css';
</script>

<template>
  <router-view v-slot="{ Component, route }">
    <transition>
      <keep-alive :include="['List']">
        <component :is="Component" :key="route.fullPath"></component>
      </keep-alive>   
    </transition>
  </router-view>
</template>

<style scoped></style>
