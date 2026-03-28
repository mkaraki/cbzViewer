<script lang="ts" setup>
import {nextTick, onBeforeMount, onBeforeUnmount, onMounted, type Ref, ref, useTemplateRef, watch} from "vue";
import '../style/read.css';
import * as Sentry from "@sentry/vue";
import PQueue from 'p-queue';
import {loadQueuedImage, resetThumbnailBatch, unloadQueuedImages} from "@/utils/queued-image-fetch.ts";

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

async function addQueuedImages() {
  Array.from(document.getElementsByClassName("queue-img") as HTMLCollectionOf<HTMLImageElement>).forEach(img => {
    loadQueuedImage(img, queue, thumbnailBatch);
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
          } else {
            setPage(1);
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
  if (lastPageMode !== null) {
    pageMode.value = lastPageMode;
  }
});

const isRtL = ref(false);

const pages = useTemplateRef('pages')

const pageNumber = ref(1);

const showingPageIds = ref([-1]);
const showingPages = ref([null]);

const getPage = () => pageNumber.value;
const setPage = (page: Number) => {
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
  if (page <= 1)
    return;
  setPage(page - getPageDecrementAmount());
}

const chPageInc = () => {
  const page = getPage();
  if (page >= data.value['pageCnt'])
    return;
  setPage(page + getPageIncrementAmount());
}

const pageSelect = () => {
  const pageStr = getPage().toString();
  const page = parseInt(prompt("Page?", pageStr) ?? pageStr);
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

  switch (pageMode.value) {
    case 'double':
      if (getPage() % 2 === 1) {
        setPage(getPage() + 1);
      }
      break;
    case 'double-except-first':
      if (getPage() % 2 === 0 && getPage() !== 1) {
        setPage(getPage() + 1);
      }
      break;
  }

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
          <img v-if="getPageAmount() === 1" class="single-page"
               src="/assets/loading.jpg"
              :data-src="`/api/img?path=${ encodeURI(data['path']) }&f=${ encodeURI(showingPages?.[0]?.['imageFile'] ?? '') }`"
          />
          <template v-else-if="getPageAmount() === 2">
            <template v-if="!isRtL">
              <img class="double-page" src="/assets/loading.jpg" :data-src="`/api/img?path=${ encodeURI(data['path']) }&f=${ encodeURI(showingPages?.[0]?.['imageFile'] ?? '') }`" />
              <img class="double-page" src="/assets/loading.jpg" :data-src="`/api/img?path=${ encodeURI(data['path']) }&f=${ encodeURI(showingPages?.[1]?.['imageFile'] ?? '') }`" />
            </template>
            <template v-else-if="isRtL">
              <img class="double-page" src="/assets/loading.jpg" :data-src="`/api/img?path=${ encodeURI(data['path']) }&f=${ encodeURI(showingPages?.[1]?.['imageFile'] ?? '') }`" />
              <img class="double-page" src="/assets/loading.jpg" :data-src="`/api/img?path=${ encodeURI(data['path']) }&f=${ encodeURI(showingPages?.[0]?.['imageFile'] ?? '') }`" />
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
