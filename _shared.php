<?php
require_once __DIR__ . '/_config.php';

define('DATA_ROOT_ABSOLUTE', realpath(DATA_ROOT));
$path = str_replace('\\', '/', $_GET['path'] ?? '/');

define('DATA_QUERY_PATH', realpath(DATA_ROOT_ABSOLUTE . $path));

if (!str_starts_with(DATA_QUERY_PATH, DATA_ROOT_ABSOLUTE)) {
    die('Query invalid');
}

function filterImageFiles(string $v): bool
{
    return preg_match('/^[a-z0-9\-_]+\.(jpg|jpeg|png|gif|tiff|webp)$/i', $v);
}

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
        case 'webp':
            return 'image/webp';
        default:
            return 'application/octet-stream';
    }
}

function fileMTimeMod($file, $serverDirective)
{
    $filemtime = filemtime($file);
    $curFileDt_str = gmdate('D, d M Y H:i:s', $filemtime) . ' GMT';
    if (isset($serverDirective['HTTP_IF_MODIFIED_SINCE'])) {
        if ($serverDirective['HTTP_IF_MODIFIED_SINCE'] === $curFileDt_str) {
            header('HTTP/1.1 304 Not Modified');
            exit;
        }
    } else {
        header('Last-Modified: ' . $curFileDt_str);
    }
}
