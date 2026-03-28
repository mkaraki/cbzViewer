<?php
require_once __DIR__ . '/../internals/_init.php';

$transaction = init_sentry_tracing('/api/read');

$path = check_path_query();
$real_path = get_real_path($path);

if ($real_path === false) {
    http_response_code(400);
    die('Invalid path');
}

$virtual_path = get_virtual_path($real_path);

if ($virtual_path === false) {
    http_response_code(400);
    die('Unable to find relative path');
}

if (!is_file($real_path)) {
    http_response_code(404);
    die('Queried directory not found');
}

$parent_dir = get_parent_if_exists($virtual_path);

if ($parent_dir === false) {
    http_response_code(500);
    die('Unexpected error: All files must belongs to a directory');
}

$ret = [
    'comicTitle' => '',
    'pages' => [],
    'path' => $virtual_path,
    'parentDir' => $parent_dir,
];

$comic_data = get_comic_data($real_path);
if ($comic_data === false) {
    http_response_code(500);
    die('Unable to read comic data. Unsupported file or unable to read.');
}

$ret['pages'] = $comic_data['pages'];
$ret['pageCnt'] = $comic_data['pageCnt'];
// ToDo: Parse ComicInfo.xml and get comic title.

header('Content-Type: application/json');
print(json_encode($ret));
$transaction->finish();
