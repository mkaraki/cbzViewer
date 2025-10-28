<script lang="ts" setup>
import {onBeforeMount, onMounted, type Ref, ref, useTemplateRef} from "vue";
import '../style/read.css';
import * as Sentry from "@sentry/vue";

const data: Ref<any> = ref([]);

const props = defineProps({
  path: String
});

// 0: Loading
// 1: failed/Not found
// 2: success
const state = ref(0);

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
  document.getElementById((page - 1).toString())?.scrollIntoView();
  setPage(page - 1);
}

const chPageInc = () => {
  const page = getPage();
  if (page >= data.value['pageCnt'])
    return;
  document.getElementById((page + 1).toString())?.scrollIntoView();
  setPage(page + 1);
}

const pageSelect = () => {
  const pageStr = getPage().toString();
  const page = parseInt(prompt("Page?", pageStr) ?? pageStr);
  document.getElementById((page).toString())?.scrollIntoView();
  setPage(page);
}
</script>

<template>
  <template v-if="state === 2">
    <header>
      <div>
        <router-link :to="`list?path=${data['parentDir']}`">Back</router-link>
      </div>
      <div>
        {{ data['comicTitle'] }}
      </div>
    </header>
    <div class="page-container">
      <div class="page-img-list-container">
        <div v-for="page in data['pages']" :id="page['pageNo']" :key="page['pageNo']" class="page-img-container">
          <img ref="pages" :alt="`Image of page ${page['pageNo']}`"
               :loading="( page['pageNo'] === 1 ? 'eager' : 'lazy' )"
               :src="`/api/img?path=${ data['path'] }&f=${ page['imageFile'] }`" class="page" />
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
        <a ref="pgNum" href="javascript:void(0)" v-on:click="pageSelect()">1</a> / {{ data['pageCnt'] }}
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
