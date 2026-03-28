<?php
require_once __DIR__ . '/../internals/_init.php';

$transaction = init_sentry_tracing('/api/thumb');

$path = check_path_query();
[$real_path, $virtual_path] = get_real_and_virtual_path_from_path($path, $transaction);

if (!is_file($real_path)) {
    http_response_code(404);
    $transaction->finish();
    die('Queried file not found');
}

process_last_modified($real_path);

$comic_data = get_comic_data($real_path);
if ($comic_data === false) {
    http_response_code(500);
    $transaction->finish();
    die('Unable to get comic data');
}

if (count($comic_data['pages']) < 1) {
    http_response_code(404);
    $transaction->finish();
    die('Page count is 0.');
}

$first_page = $comic_data['pages'][0];

$url_safe_path = urlencode($virtual_path);
$url_safe_inner_path = urlencode($first_page['imageFile']);

http_response_code(302);
header('Location: /api/img?thumb=1&path=' . $url_safe_path . '&f=' . $url_safe_inner_path);

$transaction->finish();