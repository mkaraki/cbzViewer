<?php
require_once __DIR__ . '/_shared.php';
require_once __DIR__ . '/vendor/autoload.php';

$page = intval($_GET['p'] ?? '1');

if (!is_file(DATA_QUERY_PATH)) {
    die('Not a file');
}

fileMTimeMod(DATA_QUERY_PATH, $_SERVER);

$small_query_path = strtolower(DATA_QUERY_PATH);

if (str_ends_with($small_query_path, '.cbz')) {
    $zipFile = new \PhpZip\ZipFile();
    $zipFile->openFile(DATA_QUERY_PATH);

    $fileList = array_filter($zipFile->getListFiles(), 'filterImageFiles');

    if ($page > count($fileList)) {
        die('No data');
    }
    $fileData = $fileList[$page - 1];
    header('Content-type: ' . getMimeFromExt($fileData));
    print($zipFile->getEntryContents($fileData));

    $zipFile->close();
} else if (str_ends_with($small_query_path, '.pdf')) {
    $pdfFile = new imagick(DATA_QUERY_PATH . '[' . $page - 1 . ']');
    $pdfFile->setImageFormat('webp');
    header('Content-type: image/webp');
    print($pdfFile);
    //$pdfFile->close();
} else {
    http_response_code(400);
    die('Not a comic');
}
