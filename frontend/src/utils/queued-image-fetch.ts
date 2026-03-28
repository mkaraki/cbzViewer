import lozad from "lozad";
import * as Sentry from "@sentry/vue";
import {nextTick, onBeforeMount, watch} from "vue";
import type PQueue from "p-queue";

async function loadQueuedImage(imgElement: HTMLImageElement, queue: PQueue, thumbnailBatch: AbortController) {
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

function resetThumbnailBatch(queue: PQueue, thumbnailBatch: AbortController): void {
    queue.clear();
    thumbnailBatch.abort(); // Cancel any ongoing fetches
}

function unloadQueuedImages() {
    document.querySelectorAll('.loaded.queue-img').forEach((e) => {
        const el = e as HTMLImageElement;

        URL.revokeObjectURL(el.src);
        el.src = '/assets/loading.jpg';
        el.classList.remove('loaded');
    });
}

export {
    loadQueuedImage,
    resetThumbnailBatch,
    unloadQueuedImages,
}
