<script lang="ts" setup>
import {onBeforeMount, type Ref, ref, watch} from "vue";
import * as Sentry from '@sentry/vue';
import '../style/list.css';

const data: Ref<any> = ref([]);

const props = defineProps({
  path: String
});

// 0: Loading
// 1: failed/Not found
// 2: success
const state = ref(0);

const funcOnBeforeMount = () => {
  const traceData = Sentry.getTraceData();

  state.value = 0;
  fetch(`/api/list?path=${encodeURIComponent(props.path ?? '')}`, {
    headers: {
      "sentry-trace": traceData['sentry-trace'] ?? '',
      "baggage": traceData['baggage'] ?? '',
    }
  })
      .then(v => v.json())
      .then(v => {
        data.value = v;
        state.value = 2;
      })
      .catch(e => {
        console.error(e);
        state.value = 1;
      });
};

onBeforeMount(funcOnBeforeMount);

watch(() => props.path, () => {
  funcOnBeforeMount();
})
</script>

<template>
  <template v-if="state === 2">
    <div class="container">
      <div class="row">
        <div class="col">
          <h1>Index of {{ data['currentDir'] }}</h1>
        </div>
      </div>
      <div v-if="data['hasParent']" class="row">
        <div class="col">
          <router-link :to="`/list?path=${ encodeURIComponent(data['parentDir']) }`">Parent dir</router-link>
        </div>
      </div>
      <div class="row">
        <div class="col">
          <div v-for="item in data['items']" :key="item.path" class="item">
            <template v-if="item['isDir']">
              <router-link :to="`/list?path=${ encodeURIComponent(item['path']) }`">
                <img :alt="`Thumbnail of ${ item['name'] }`" :src="`/api/thumb?path=${ encodeURIComponent(item['thumbPath'])}`" class="thumb">
              </router-link>
            </template>
            <template v-else>
              <router-link :to="`/read?path=${ encodeURIComponent(item['path']) }`">
                <img :alt="`Thumbnail of ${ item['name'] }`" :src="`/api/thumb?path=${ encodeURIComponent(item['thumbPath'])}`" class="thumb">
              </router-link>
            </template>
            <div class="card-body">
              <template v-if="item['isDir']">
                <h5 class="card-title">
                  <router-link :to="`/list?path=${ encodeURIComponent(item['path']) }`">{{ item['name'] }}</router-link>
                </h5>
              </template>
              <template v-else>
                <h5 class="card-title">
                  <router-link :to="`/read?path=${ encodeURIComponent(item['path']) }`">{{ item['name'] }}</router-link>
                </h5>
              </template>
            </div>
          </div>
        </div>
      </div>
    </div>
  </template>
  <template v-else-if="state === 0">
    <div class="container">
      <div class="row">
        <div class="col">
          <a href="javascript:void(0)" onclick="history.back()">Cancel load</a>
        </div>
      </div>
      <div class="row">
        <div class="col">
          <div v-for="item in 5" :key="item" aria-hidden="true" class="card">
            <!-- ToDo: Placeholder image -->
            <div class="card-body placeholder-glow">
              <h5 class="card-title placeholder-glow">
                <span class="placeholder col-6"></span>
              </h5>
            </div>
          </div>
        </div>
      </div>
    </div>
  </template>
  <template v-else-if="state === 1">
    <div class="container">
      <div class="row">
        <div class="col">
          <a href="javascript:void(0)" onclick="history.back()">Back to previous page</a>
        </div>
      </div>
      <div class="row">
        <div class="col">
          Not found or error.
        </div>
      </div>
    </div>
  </template>
  <footer>
    <hr />
    <a href="/legal" rel="noopener noreferrer" target="_blank">Legal</a>
  </footer>
</template>