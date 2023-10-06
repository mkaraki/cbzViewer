<?php
require_once __DIR__ . '/_shared.php';
require_once __DIR__ . '/vendor/autoload.php';

$page = intval($_GET['p'] ?? '1');

if (!is_file(DATA_QUERY_PATH)) {
    die('Not a file');
}

fileMTimeMod(DATA_QUERY_PATH, $_SERVER);

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
