<?php
if (!is_file(__DIR__ . '/config.json')) {
    http_response_code(500);
    die("Configuration error: missing config file");
}

$config = json_decode(file_get_contents(__DIR__ . '/config.json'), true);
if ($config === null) {
    http_response_code(500);
    die("Configuration error: invalid config file");
}
if (!isset($config['cbzDir'])) {
    http_response_code(500);
    die("Configuration error: missing config");
}
define("CBZ_DIR", $config['cbzDir']);