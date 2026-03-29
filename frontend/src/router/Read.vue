<script lang="ts" setup>
import {nextTick, onBeforeMount, onBeforeUnmount, onMounted, type Ref, ref, watch} from "vue";
import '../style/read.css';
import * as Sentry from "@sentry/vue";
import PQueue from 'p-queue';
import {resetThumbnailBatch} from "@/utils/queued-image-fetch.ts";

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

const pageSrc: Ref<any> = ref([]);

async function loadQueuedImage(pageNo: number, imageFile: string) {
  const src = `/api/img?path=${encodeURIComponent(data.value['path'])}&f=${encodeURIComponent(imageFile)}`;

  // Add the fetch operation to the queue
  await queue.add(async () => {
    if (typeof pageSrc.value[pageNo] !== "undefined" && pageSrc.value[pageNo] !== "/assets/loading.jpg") {
      return;
    }
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
        pageSrc.value[pageNo] = '/assets/error.jpg';
        throw new Error('Network response was not ok');
      }

      // Convert the raw response into a local browser Blob URL
      const blob = await response.blob();
      pageSrc.value[pageNo] = URL.createObjectURL(blob);
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') return;
      pageSrc.value[pageNo] = '/assets/error.jpg';
      console.error("Failed to load thumbnail:", error);
    }
  });
}

function unloadQueuedImages() {
  pageSrc.value.forEach((e) => {
    URL.revokeObjectURL(e);
  });
  pageSrc.value = [];
}

async function addQueuedImages() {
  if (state.value !== 2) return;
  
  data.value['pages'].forEach((page) => {
    if (pageSrc.value[page['pageNo']] === undefined) {
      pageSrc.value[page['pageNo']] = '/assets/loading.jpg';
    }
    loadQueuedImage(page['pageNo'], page['imageFile']);
  })
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
            console.debug('Trying to set page:', pageStr[1]);
            const page = parseInt(pageStr[1]);
            setPage(page);
          } else {
            setPage(1);
          }
        }, 0);
      })
      .catch(e => {
        console.error(e);
        state.value = 1;
      });
})

const resetThumbnailBatchProcess = () => {
  resetThumbnailBatch(queue, thumbnailBatch);
  thumbnailBatch = new AbortController();
  unloadQueuedImages();
}

watch(data, async () => {
  await nextTick();
  await addQueuedImages();
}, { immediate: true });

const onBeforeUnmountFunction = () => {
  resetThumbnailBatchProcess();
  document.onkeydown = null;
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

  const lastPageMode = localStorage.getItem('pageMode')
  if (lastPageMode !== null && ['single', 'double', 'double-except-first'].includes(lastPageMode)) {
    pageMode.value = lastPageMode;
  }
});

const isRtL = ref(false);

const pageNumber = ref(1);

const showingPageIds = ref([-1]);
const showingPages = ref([null]);

const getPage = () => pageNumber.value;
const setPage = (page: number) => {
  pageNumber.value = page;
  window.history.replaceState({}, '', `#${pageNumber.value}`);
  showingPageIds.value = getToShowImageRealNo(page);
  for (let i = 0; i < showingPageIds.value.length; i++) {
    showingPages.value[i] = data.value['pages']?.[showingPageIds.value[i] - 1];
  }
  console.debug(
    page,
    showingPageIds.value,
    showingPages.value,
  );
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

const getPageIncrementAmount = () => {
  switch (pageMode.value) {
    case 'single':
    default:
      return 1;
    case 'double':
      return 2;
    case 'double-except-first':
      if (getPage() === 1) {
        return 1;
      }
      return 2;
  }
}

const getPageDecrementAmount = () => {
  switch (pageMode.value) {
    case 'single':
    default:
      return 1;
    case 'double':
      return 2;
    case 'double-except-first':
      if (getPage() === 2) {
        return 1;
      }
      return 2;
  }
}

const chPageDec = () => {
  const page = getPage();
  const decAmount = getPageDecrementAmount();
  if (page - decAmount < 1)
    return;
  setPage(Math.max(1, page - decAmount));
}

const chPageInc = () => {
  const page = getPage();
  const incAmount = getPageIncrementAmount();
  const pageCnt = data.value['pageCnt'];
  if (page >= pageCnt)
    return;
  setPage(Math.min(pageCnt, page + incAmount));
}

const pageSelect = () => {
  const pageStr = getPage().toString();
  const page = parseInt(prompt("Page?", pageStr) ?? pageStr);
  if (Number.isNaN(page) || page < 1 || page > data.value['pageCnt'])
    return;
  setPage(page);
}

const pageMode = ref('single');

const pageModeSwitch = () => {
  switch (pageMode.value) {
    case 'single':
      pageMode.value = 'double';
      break;
    case 'double':
      pageMode.value = 'double-except-first';
      break;
    case 'double-except-first':
      pageMode.value = 'single';
      break;
    default:
      pageMode.value = 'single';
      break;
  }

  // Always recompute the currently shown page(s) after changing pageMode.
  // Start from the current page and adjust for parity rules if necessary.
  let newPage = getPage();

  switch (pageMode.value) {
    case 'double':
      if (newPage % 2 === 0) {
        newPage = newPage - 1;
      }
      break;
    case 'double-except-first':
      if (newPage % 2 === 0 && newPage !== 1) {
        newPage = newPage + 1;
      }
      break;
  }

  setPage(newPage);
  localStorage.setItem('pageMode', pageMode.value);
}

const getPageAmount = (): number => {
  switch (pageMode.value) {
    case 'single':
    default:
      return 1;
    case 'double':
      return 2;
    case 'double-except-first':
      return 2;
  }
}

const getToShowImageRealNo = (currentPage: number): Array<number> => {
  const mode = pageMode.value;
  switch (mode) {
    case 'single':
    default:
      return [currentPage];
    case 'double': {
      return [currentPage, currentPage + 1];
    }
    case 'double-except-first': {
      if (currentPage === 1) {
        return [-1, 1];
      }
      return [currentPage, currentPage + 1];
    }
  }
}

watch(pageMode, async () => {
  await nextTick();
  await addQueuedImages();
});
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
        <div class="page-img-container">
          <img v-if="getPageAmount() === 1" class="queue-img single-page"
              :src="(pageSrc[showingPages?.[0]?.['pageNo'] ?? -1] ?? '')"
          />
          <template v-else-if="getPageAmount() === 2">
            <template v-if="!isRtL">
              <img class="queue-img double-page" :src="(pageSrc[showingPages?.[0]?.['pageNo'] ?? -1] ?? '')" />
              <img class="queue-img double-page" :src="(pageSrc[showingPages?.[1]?.['pageNo'] ?? -1] ?? '')" />
            </template>
            <template v-else-if="isRtL">
              <img class="queue-img double-page" :src="(pageSrc[showingPages?.[1]?.['pageNo'] ?? -1] ?? '')" />
              <img class="queue-img double-page" :src="(pageSrc[showingPages?.[0]?.['pageNo'] ?? -1] ?? '')" />
            </template>
          </template>
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
        <a id="pgNum" ref="pgNum" href="javascript:void(0)" v-on:click="pageSelect()">{{ pageNumber }}</a> / {{ data['pageCnt'] }}
        <a href="javascript:void(0)" v-on:click="rtlSwitch()">{{ ( isRtL ? 'RtL' : 'LtR' ) }}</a> |
        <a href="javascript:void(0)" v-on:click="pageModeSwitch()">{{ pageMode }}</a>
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
