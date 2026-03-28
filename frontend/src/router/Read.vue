<script lang="ts" setup>
import {nextTick, onBeforeMount, onBeforeUnmount, onMounted, type Ref, ref, useTemplateRef, watch} from "vue";
import '../style/read.css';
import * as Sentry from "@sentry/vue";
import PQueue from 'p-queue';
import lozad from "lozad";

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
      // The queue ensures only limited number of these fetches are ever running at once
      const traceData = Sentry.getTraceData();
      const response = await fetch(src, {
        signal: thumbnailBatch.signal,
        headers: {
          "sentry-trace": traceData['sentry-trace'] ?? '',
          "baggage": traceData['baggage'] ?? '',
        }
      });

      if (!response.ok) {
        imgElement.src = '/assets/error.jpg';
        throw new Error('Network response was not ok');
      }

      // Convert the raw response into a local browser Blob URL
      const blob = await response.blob();
      imgElement.src = URL.createObjectURL(blob);

      imgElement.classList.add('loaded');
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') return;
      imgElement.src = '/assets/error.jpg';
      console.error("Failed to load thumbnail:", error);
    }
  });
}

async function addQueuedImages() {
  Array.from(document.getElementsByClassName("queue-img") as HTMLCollectionOf<HTMLImageElement>).forEach(img => {
    loadQueuedImage(img);
  });
}

onBeforeMount(() => {
  const traceData = Sentry.getTraceData();

  fetch(`/api/read?path=${encodeURIComponent(props.path ?? '')}`, {
    headers: {
      "sentry-trace": traceData['sentry-trace'] ?? '',
      "baggage": traceData['baggage'] ?? '',
    }
  })
      .then(v => v.json())
      .then(v => {
        data.value = v;
        state.value = 2;

        setTimeout(() => {
          let pageStr: RegExpMatchArray|null = location.hash.match(/^#(\d+)$/);
          if (pageStr !== null && typeof pageStr[1] === 'string') {
            console.trace('Trying to set page:', pageStr[1]);
            const page = parseInt(pageStr[1]);
            setPage(page);
          }

          console.trace("Changing images to eager: ", pages.value);
          pages.value?.forEach((v: Element) => {
            (v as HTMLImageElement).loading = 'eager';
          });
        }, 0);
      })
      .catch(e => {
        console.error(e);
        state.value = 1;
      });
})

function resetThumbnailBatch() {
  queue.clear();
  thumbnailBatch.abort();
  thumbnailBatch = new AbortController();
}

watch(data, async () => {
  await nextTick();
  await addQueuedImages();
}, { immediate: true });

function unloadQueuedImages() {
  document.querySelectorAll('.loaded.queue-img').forEach((e) => {
    const el = e as HTMLImageElement;

    URL.revokeObjectURL(el.src);
    el.src = '/assets/loading.jpg';
    el.classList.remove('loaded');
  });
}

const onBeforeUnmountFunction = () => {
  resetThumbnailBatch();
  unloadQueuedImages();
}

onBeforeUnmount(onBeforeUnmountFunction);

onMounted(() => {
  document.onkeydown = (e) => {
    switch(e.key)
    {
      case 'PageUp':
      case 'ArrowUp':
        chPageDec();
        break;

      case 'ArrowDown':
      case 'PageDown':
        chPageInc();
        break;

      case 'ArrowLeft':
        leftHandler();
        break;

      case 'ArrowRight':
        rightHandler();
        break;
    }
  }

  const lastIsRtL = localStorage.getItem('isRtL')
  if (lastIsRtL !== null) {
    isRtL.value = lastIsRtL === 'true';
  }
});

const isRtL = ref(false);

const pages = useTemplateRef('pages')
const pgNum = useTemplateRef('pgNum')

const getPage = () => parseInt(pgNum.value?.innerText ?? '1');
const setPage = (page: Number) => {
  if (pgNum.value !== null) {
    pgNum.value.innerText = page.toString();
  }
  document.getElementById((page).toString())?.scrollIntoView();
  window.history.replaceState({}, '', `#${page}`);
};

const rtlSwitch = () => {
  isRtL.value = !isRtL.value;

  localStorage.setItem('isRtL', isRtL.value ? 'true' : 'false');
}

const rightHandler = () => {
  if (isRtL.value) {
    chPageDec();
  } else {
    chPageInc();
  }
}

const leftHandler = () => {
  if (isRtL.value) {
    chPageInc();
  } else {
    chPageDec();
  }
}

const chPageDec = () => {
  const page = getPage();
  if (page <= 1)
    return;
  setPage(page - 1);
}

const chPageInc = () => {
  const page = getPage();
  if (page >= data.value['pageCnt'])
    return;
  setPage(page + 1);
}

const pageSelect = () => {
  const pageStr = getPage().toString();
  const page = parseInt(prompt("Page?", pageStr) ?? pageStr);
  setPage(page);
}
</script>

<template>
  <template v-if="state === 2">
    <header>
      <div>
        <router-link :to="`list?path=${ encodeURI(data['parentDir']) }`">Back</router-link>
      </div>
      <div>
        {{ data['comicTitle'] }}
      </div>
    </header>
    <div class="page-container">
      <div class="page-img-list-container">
        <div v-for="page in data['pages']" :id="page['pageNo']" :key="page['pageNo']" class="page-img-container">
          <img ref="pages" :alt="`Image of page ${page['pageNo']}`"
               src="/assets/loading.jpg"
               :data-src="`/api/img?path=${ encodeURI(data['path']) }&f=${ encodeURI(page['imageFile']) }`" class="page queue-img" />
        </div>
      </div>
      <a class="prev-controller" href="javascript:void(0)" v-on:click="leftHandler()"></a>
      <a class="next-controller" href="javascript:void(0)" v-on:click="rightHandler()"></a>
    </div>
    <footer>
      <div>
        <a href="javascript:void(0)" v-on:click="leftHandler()">{{ isRtL ? 'Next' : 'Prev' }}</a>
      </div>
      <div>
        <a id="pgNum" ref="pgNum" href="javascript:void(0)" v-on:click="pageSelect()">1</a> / {{ data['pageCnt'] }}
        <a href="javascript:void(0)" v-on:click="rtlSwitch()">{{ ( isRtL ? 'RtL' : 'LtR' ) }}</a>
      </div>
      <div>
        <a href="javascript:void(0)" v-on:click="rightHandler()">{{ isRtL ? 'Prev' : 'Next' }}</a>
      </div>
    </footer>
  </template>
  <template v-else-if="state === 0">
    <div class="container">
      <div class="row">
        <div class="col">
          <a href="javascript:void(0)" onclick="history.back()">Force cancel</a>
        </div>
      </div>
      <div class="row">
        <div class="col">
          <div class="d-flex justify-content-center m-5">
            <div class="spinner-border" role="status">
              <span class="visually-hidden">Loading...</span>
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
</template>
