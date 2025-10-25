<script lang="ts" setup>
import 'bootstrap/scss/bootstrap.scss';
import './style/global.css';
</script>

<template>
  <transition name="fade">
    <router-view></router-view>
  </transition>
</template>

<style scoped></style>
