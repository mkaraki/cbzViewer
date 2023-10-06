<?php
require_once __DIR__ . '/_config.php';

define('DATA_ROOT_ABSOLUTE', realpath(DATA_ROOT));
$path = str_replace('\\', '/', $_GET['path'] ?? '/');

define('DATA_QUERY_PATH', realpath(DATA_ROOT_ABSOLUTE . $path));

if (!str_starts_with(DATA_QUERY_PATH, DATA_ROOT_ABSOLUTE)) {
    die('Query invalid');
}
