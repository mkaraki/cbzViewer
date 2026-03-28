<?php
require_once __DIR__ . '/../internals/_init.php';

$transaction = init_sentry_tracing('/api/thumb_dir');

$path = check_path_query();
[$real_path, $virtual_path] = get_real_and_virtual_path_from_path($path, $transaction);

if (!is_dir($real_path)) {
    http_response_code(404);
    $transaction->finish();
    die('Queried directory not found');
}

process_last_modified($real_path);

$get_first_child_item = search_supported_item_in_sub_dirs($real_path);
if ($get_first_child_item === false) {
    http_response_code(404);
    $transaction->finish();
    die('There are no supported item');
}

$virtual_item_path = get_virtual_path($get_first_child_item);
$url_virtual_item = urlencode($virtual_item_path);

http_response_code(302);
header('Location: /api/thumb?path=' . $url_virtual_item);

$transaction->finish();