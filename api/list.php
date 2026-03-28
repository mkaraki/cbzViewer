<?php
require_once __DIR__ . '/../internals/_init.php';

$transaction = init_sentry_tracing('/api/list');

$path = check_path_query();
$real_path = get_real_path($path);

if ($real_path === false) {
    http_response_code(400);
    $transaction->finish();
    die('Invalid path');
}

$virtual_path = get_virtual_path($real_path);

if ($virtual_path === false) {
    http_response_code(400);
    $transaction->finish();
    die('Unable to find relative path');
}

if (!is_dir($real_path)) {
    http_response_code(404);
    $transaction->finish();
    die('Queried directory not found');
}

if (!str_ends_with($real_path, '/') && !str_ends_with($real_path, '\\')) {
    $real_path .= DIRECTORY_SEPARATOR;
}

process_last_modified($real_path);

$items = scandir($real_path, SCANDIR_SORT_NONE);
if ($items === false) {
    http_response_code(500);
    $transaction->finish();
    die('Unable to retrieve directory items');
}
natsort($items);

$ret_items = [];

foreach ($items as $item) {
    if ($item === '.' || $item === '..') {
        continue;
    }
    
    $real_item_path = $real_path . $item;
    
    // Verbose. But for secure purpose
    if (!is_safe_path($real_item_path)) {
        frankenphp_log(
            "There are unsafe path file in safe directory: Unsafe: " . $real_item_path,
            FRANKENPHP_LOG_LEVEL_WARN,
        );
        continue;
    }
    
    $virtual_item_path = get_virtual_path($real_item_path);
    if ($virtual_item_path === false) {
        frankenphp_log(
            "Unable to retrieve virtual path of " . $real_item_path,
            FRANKENPHP_LOG_LEVEL_WARN,
        );
        continue;
    }
    
    $ret_items[] = [
        'name' => $item,
        'path' => $virtual_item_path,
        'isDir' => is_dir($real_item_path),
    ];
}

$ret['items'] = $ret_items;

$parent_path = get_parent_if_exists($virtual_path);
if ($parent_path === false) {
    $ret['hasParent'] = false;
    $ret['parentDir'] = '';
} else {
    $ret['hasParent'] = true;
    $ret['parentDir'] = $parent_path;
}

header('Content-type: application/json');
print json_encode($ret);
$transaction->finish();
