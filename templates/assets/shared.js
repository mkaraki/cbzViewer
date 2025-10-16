let sentryTraceHeader = undefined;
let sentryBaggageHeader = undefined;

try {
    const traceData = Sentry.getTraceData();
    sentryTraceHeader = traceData["sentry-trace"];
    sentryBaggageHeader = traceData["baggage"];
} catch (e) {
    console.error('Unable to retrive Sentry trace info', e);
}

document.addEventListener('lazybeforeunveil', (e) => {
    const elem = e.target;
    const fetchUrl = elem.dataset.srcFetch;
    if (!fetchUrl || elem.dataset.blobFetched) return;

    e.preventDefault();

    fetch(fetchUrl, {
        method: "GET",
        headers: {
            baggage: sentryBaggageHeader,
            "sentry-trace": sentryTraceHeader,
        },
    })
        .then((response) => {
            if (!response.ok) {
                console.error('Unable to get response', e);
                throw new Error('Failed to retrieve response');
            }
            return response.blob();
        })
        .then((blob) => {
            const blobUrl = URL.createObjectURL(blob);
            elem.dataset.src = blobUrl;
            elem.dataset.blobFetched = true;
            lazySizes.loader.unveil(elem);

            elem.addEventListener('load', function onLoad(e) {
               URL.revokeObjectURL(blobUrl);
               elem.removeEventListener('load', onLoad);
            });
        })
        .catch((err) => {
            console.error('Unable to process image fetch', e);
        });
});
