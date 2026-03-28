type ReadablePage = {
    pageNo: number,
    imageFile: string,
}

type ReadableItem = {
    comicTitle: string,
    pages: ReadablePage[],
    pageCnt: number,
    path: string,
    parentDir: string,
};

export type {
    ReadablePage,
    ReadableItem,
}