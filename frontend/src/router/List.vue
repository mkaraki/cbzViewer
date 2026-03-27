<script lang="ts" setup>
import {nextTick, onBeforeMount, onBeforeUnmount, type Ref, ref, watch} from "vue";
import * as Sentry from '@sentry/vue';
import '../style/list.css';
import PQueue from 'p-queue';

defineOptions({
  name: 'List',
});

const data: Ref<any> = ref([]);

const props = defineProps({
  path: String
});

// 0: Loading
// 1: failed/Not found
// 2: success
const state = ref(0);

const queue = new PQueue({ concurrency: 2 });
let thumbnailBatch = new AbortController();

async function loadQueuedImage(imgElement: HTMLImageElement) {
  const src = imgElement.dataset.src;
  if (!src) return;

  // Add the fetch operation to the queue
  await queue.add(async () => {
    if (imgElement.classList.contains('loaded')) return; // Skip if already loaded
    if (!imgElement.isConnected) return;

    try {
      // The queue ensures only 4 of these fetches are ever running at once
      const traceData = Sentry.getTraceData();
      const response = await fetch(src, {
        signal: thumbnailBatch.signal,
        headers: {
          "sentry-trace": traceData['sentry-trace'] ?? '',
          "baggage": traceData['baggage'] ?? '',
        }
      });

      if (!response.ok) throw new Error('Network response was not ok');

      // Convert the raw response into a local browser Blob URL
      const blob = await response.blob();
      imgElement.src = URL.createObjectURL(blob);

      imgElement.classList.add('loaded');
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') return;
      console.error("Failed to load thumbnail:", error);
    }
  });
}

async function addQueuedImages() {
  Array.from(document.getElementsByClassName("queue-img") as HTMLCollectionOf<HTMLImageElement>).forEach(img => {
    loadQueuedImage(img);
  });
}

const funcOnBeforeMount = () => {
  if (state.value !== 2)
    state.value = 0;
  
  const traceData = Sentry.getTraceData();
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

function resetThumbnailBatch() {
  queue.clear();
  thumbnailBatch.abort();
  thumbnailBatch = new AbortController();
}

watch(() => props.path, () => {
  resetThumbnailBatch();
  funcOnBeforeMount();
})

watch(data, async () => {
  await nextTick();
  await addQueuedImages();
}, { immediate: true });

function unloadQueuedImages() {
  document.querySelectorAll('.loaded .queue-img').forEach((e) => {
    const el = e as HTMLImageElement;

    URL.revokeObjectURL(el.src);
    el.classList.remove('loaded')
  });
}

const onBeforeUnmountFunction = () => {
  resetThumbnailBatch();
  unloadQueuedImages();
}

onBeforeUnmount(onBeforeUnmountFunction);
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
                <img :alt="`Thumbnail of ${ item['name'] }`" src="" :data-src="`/api/thumb_dir?path=${ encodeURIComponent(item['path'])}`" class="thumb queue-img" loading="lazy">
              </router-link>
            </template>
            <template v-else>
              <router-link :to="`/read?path=${ encodeURIComponent(item['path']) }`">
                <img :alt="`Thumbnail of ${ item['name'] }`" src="" :data-src="`/api/thumb?path=${ encodeURIComponent(item['path'])}`" class="thumb queue-img" loading="lazy">
              </router-link>
            </template>
            <div class="card-body">
              <template v-if="item['isDir']">
                <h5 class="card-title">
                  <router-link :to="`/list?path=${ encodeURIComponent(item['path']) }`">{{ item['name'] }}/</router-link>
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