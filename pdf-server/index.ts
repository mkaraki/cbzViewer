import * as Sentry from "@sentry/bun";
import * as pdfjsLib from "pdfjs-dist/legacy/build/pdf.mjs";
import "./api-types";
import type {ReadableItem, ReadablePage} from "./api-types.ts";
import {
    getCanonicalCbzDir,
    getParentIfExists,
    getRealPath,
    getThumbScale,
    getThumbSize,
    getVirtualPath
} from "./functions";
import type {PDFDocumentProxy} from "pdfjs-dist/legacy/build/pdf.mjs";

// Check CBZ_DIR is OK.
{
    const cbzDirTest = getCanonicalCbzDir();
    if (cbzDirTest === false) {
        Sentry.logger.fatal("Unable to get canonicalized CBZ_DIR");
        console.error("Unable to get canonicalized CBZ_DIR");
        process.exit(1);
    }
    console.info("CBZ_DIR is set to: " + cbzDirTest);
}

const server = Bun.serve({
    port: 8081,
    // `routes` requires Bun v1.2.3+
    routes: {
        // Static routes
        "/healthz": new Response("OK"),

        "/api/pdf/read": async request => {
            const sentryTrace = request.headers.get("sentry-trace") ?? undefined;
            const baggage = request.headers.get("baggage");
            return await Sentry.continueTrace({ sentryTrace, baggage }, () => {
                return Sentry.startSpan(
                {
                    name: "/api/pdf/read",
                    op: "http.server",
                },
                async () => {
                    // Get get params `path`:
                    const path = new URL(request.url).searchParams.get("path");
                    if (path === null) {
                        return new Response("Missing 'path' query parameter", { status: 400 });
                    }
                    const realPath = getRealPath(path);
                    if (realPath === false) {
                        return new Response("Specified 'path' is not in CBZ_DIR", { status: 400 });
                    }
                    const virtualPath = getVirtualPath(realPath);
                    if (virtualPath === false) {
                        return new Response("Unable to validate path", { status: 500 });
                    }
                    
                    const parent = getParentIfExists(virtualPath);
                    if (parent === false) {
                        return new Response("Unable to get parent directory", { status: 500 });
                    }
                    
                    let pdfDocument: PDFDocumentProxy;
                    try {
                        pdfDocument = await pdfjsLib.getDocument(realPath).promise
                    } catch (error) {
                        Sentry.logger.error("Unable to read pdf file", {
                            file: realPath,
                            error: (error as Error).toString(),
                        });
                        console.error("Unable to read pdf file", realPath, error);
                        return new Response("Unable to read pdf file", { status: 500 });
                    }
                    
                    let pages: ReadablePage[] = [];
                    for (let i = 0; i < pdfDocument.numPages; ++i) {
                        let humanPageNo = i + 1;
                        pages.push({
                            pageNo: humanPageNo,
                            imageFile: humanPageNo.toString(),
                        });
                    }
                    
                    const readRet: ReadableItem = {
                        comicTitle: "",
                        pageCnt: pdfDocument.numPages,
                        pages: pages,
                        parentDir: parent,
                        path: virtualPath
                    }
                    
                    return new Response(JSON.stringify(readRet), {
                        headers: {
                            'Content-type': 'application/json',
                            'Access-Control-Allow-Origin': '*',
                            'Access-Control-Allow-Headers': 'Content-Type, sentry-trace, baggage',
                        }
                    });
                });
            });
        },
        
        "/api/pdf/img": async request => {
            const sentryTrace = request.headers.get("sentry-trace") ?? undefined;
            const baggage = request.headers.get("baggage");
            return await Sentry.continueTrace({ sentryTrace, baggage }, () => {
                return Sentry.startSpan(
                    {
                        name: "/api/pdf/img",
                        op: "http.server",
                    },
                    async () => {
                        // Validate `f` param:
                        const f = new URL(request.url).searchParams.get("f");
                        if (f === null) {
                            return new Response("Missing 'f' query parameter", { status: 400 });
                        }
                        if (!/^\d+$/.test(f)) {
                            return new Response("Invalid `f` query parameter", { status: 400 });
                        }
                        const pageNo = parseInt(f);
                        if (pageNo < 1) {
                            return new Response("Invalid `f` query parameter: Invalid range", { status: 400 });
                        }
                        
                        const thumb = new URL(request.url).searchParams.get("thumb");
                        const isThumb = thumb !== null;
                    
                        // Get get params `path`:
                        const path = new URL(request.url).searchParams.get("path");
                        if (path === null) {
                            return new Response("Missing 'path' query parameter", { status: 400 });
                        }
                        const realPath = getRealPath(path);
                        if (realPath === false) {
                            return new Response("Specified 'path' is not in CBZ_DIR", { status: 400 });
                        }
                        const virtualPath = getVirtualPath(realPath);
                        if (virtualPath === false) {
                            return new Response("Unable to validate path", { status: 500 });
                        }

                        const parent = getParentIfExists(virtualPath);
                        if (parent === false) {
                            return new Response("Unable to get parent directory", { status: 500 });
                        }

                        let pdfDocument: PDFDocumentProxy;
                        try {
                            pdfDocument = await pdfjsLib.getDocument(realPath).promise
                        } catch (error) {
                            Sentry.logger.error("Unable to read pdf file", {
                                file: realPath,
                                error: (error as Error).toString(),
                            });
                            console.error("Unable to read pdf file", realPath, error);
                            return new Response("Unable to read pdf file", { status: 500 });
                        }

                        if (pageNo > pdfDocument.numPages) {
                            return new Response("Out of page", {status: 404});
                        }

                        const canvasFactory = pdfDocument.canvasFactory;
                        
                        const page = await pdfDocument.getPage(pageNo);
                        let viewport = await page.getViewport({ scale: 1.0 });
                        
                        if (isThumb) {
                            let scale = getThumbScale(viewport.width, viewport.height);
                            
                            viewport = await page.getViewport({ scale });
                        }

                        let width = viewport.width;
                        let height = viewport.height;
                        
                        let canvasAndContext = canvasFactory.create(
                            width, 
                            height,
                        );
                        let canvas = canvasAndContext.canvas;
                        let context = canvasAndContext.context;
                        
                        try {
                            await page.render({
                                canvasContext: context,
                                viewport: viewport,
                            }).promise;
                        } catch (error) {
                            Sentry.logger.error("Unable to render PDF page", {
                                file: realPath,
                                pageNo: pageNo,
                            });
                            console.error("Unable to render PDF page", realPath, error);
                            return new Response("Unable to render PDF page", {status: 500});                            
                        }
                        
                        const image = canvas.toBuffer("image/jpeg");
                        
                        return new Response(image, {
                            headers: {
                                "Content-Type": "image/jpeg",
                                'Access-Control-Allow-Origin': '*',
                                'Access-Control-Allow-Headers': 'Content-Type, sentry-trace, baggage',
                            }
                        });
                    }
                );
            });
        },
    },
    
    websocket: {
        'open': () => {},
        'message': () => {},
        'close': () => {},
    },

    // (optional) fallback for unmatched routes:
    // Required if Bun's version < 1.2.3
    fetch(req) {
        return new Response("Not Found", { status: 404 });
    },
});

console.log(`Server running at ${server.url}`);