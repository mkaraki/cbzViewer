<?php
require_once __DIR__ . '/internals/_init.php';
$content = file_get_contents("index.html");

$content = str_replace("{{ .SentryBaggage }}", \Sentry\getBaggage(), $content);
$content = str_replace("{{ .SentryTrace }}", \Sentry\getTraceparent(), $content);
$content = str_replace("{{ .SentryDsn }}", getenv('SENTRY_DSN') ?? '', $content);
$content = str_replace("{{ .ServerHost }}", $_SERVER['HTTP_HOST'] ?? '', $content);

header('Content-type: text/html; charset=utf-8');
print $content;