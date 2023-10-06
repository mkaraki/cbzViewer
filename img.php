<?php
require_once __DIR__ . '/_shared.php';
require_once __DIR__ . '/vendor/autoload.php';

$page = intval($_GET['p'] ?? '1');

if (!is_file(DATA_QUERY_PATH)) {
    die('Not a file');
}


$zipFile = new \PhpZip\ZipFile();
$zipFile->openFile(DATA_QUERY_PATH);

$fileList = array_filter($zipFile->getListFiles(), function ($v) {
    return preg_match('/^[a-z0-9\-_]+\.(jpg|jpeg|png|gif|tiff)$/i', $v);
});

function getMimeFromExt(string $filename): string
{
    switch (strtolower(pathinfo($filename, PATHINFO_EXTENSION))) {
        case 'jpg':
        case 'jpeg':
            return 'image/jpeg';
        case 'png':
            return 'image/png';
        case 'gif':
            return 'image/gif';
        case 'tiff':
            return 'image/tiff';
        default:
            return 'application/octet-stream';
    }
}

if ($page > count($fileList)) {
    die('No data');
}
$fileData = $fileList[$page - 1];
header('Content-type: ' . getMimeFromExt($fileData));
print($zipFile->getEntryContents($fileData));

$zipFile->close();
