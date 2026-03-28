<?php
require_once __DIR__ . '/../_config.php';
require_once __DIR__ . '/../vendor/autoload.php';
require_once __DIR__ . '/comic_format/ComicFormat.php';
require_once __DIR__ . '/comic_format/CbzFile.php';

use comic_format\CbzFile;

\Sentry\init([
    // Add request headers, cookies and IP address,
    // see https://docs.sentry.io/platforms/php/data-management/data-collected/ for more info
    // Disable it due to this project is OSS.
    'send_default_pii' => false,
    // Specify a fixed sample rate
    'traces_sample_rate' => 1.0,
    // Set a sampling rate for profiling - this is relative to traces_sample_rate
    'profiles_sample_rate' => 1.0,
    // Enable logs to be sent to Sentry
    'enable_logs' => true,
]);

const SUPPORTED_IMAGE_EXTENSIONS = ['jpg', 'jpeg', 'gif', 'png', 'webp'];
const SUPPORTED_ITEM_EXTENSIONS = ['cbz'];

function get_mime_type_from_extension(string $extension): string|false {
    switch ($extension) {
        case 'jpg':
        case 'jpeg':
            return 'image/jpeg';
        case 'gif':
            return 'image/gif';
        case 'png':
            return 'image/png';
        case 'webp':
            return 'image/webp';
        default:
            return false;
    }
}

function canonicalize_cbz_dir(): string|false {
    $cbz_dir = realpath(CBZ_DIR);
    if ($cbz_dir === false) {
        return false;
    }

    if (!str_ends_with($cbz_dir, '/') && !str_ends_with($cbz_dir, '\\')) {
        $cbz_dir .= DIRECTORY_SEPARATOR;
    }
    
    return $cbz_dir;
}

function get_virtual_path(string $real_path): string|false {
    $cbz_dir = canonicalize_cbz_dir();
    if ($cbz_dir === false) {
        return false;
    }
    
    if (!is_safe_path($real_path)) {
        return false;
    }

    // This is for in case of
    // CBZ_DIR = /path/to/cbz/dir/
    // $real_path = /path/to/cbz/dir
    if (is_dir($real_path) && (!str_ends_with($real_path, '/') && !str_ends_with($real_path, '\\'))) {
        $real_path .= DIRECTORY_SEPARATOR;
    }
    
    $virtual_path = substr($real_path, strlen($cbz_dir));
    
    return $virtual_path;
}

function get_real_path(string $virtual_path): string|false {
    $cbz_dir = canonicalize_cbz_dir();
    if ($cbz_dir === false) {
        return false;
    }
    
    if (str_starts_with($virtual_path, '/')) {
        $virtual_path = substr($virtual_path, 1);
    }
    
    $real_path = $cbz_dir . $virtual_path;
    $real_path = realpath($real_path);
    
    if (!$real_path) {
        return false;
    }
    
    if (!is_safe_path($real_path)) {
        return false;
    }
    
    return $real_path;
}

function is_safe_path(string $real_path): bool {
    $real_path = realpath($real_path);
    if ($real_path === false) {
        return false;
    }
    $cbz_dir = canonicalize_cbz_dir();
    if ($cbz_dir === false) {
        return false;
    }
    
    // This is for in case of
    // CBZ_DIR = /path/to/cbz/dir/
    // $real_path = /path/to/cbz/dir
    if (is_dir($real_path) && (!str_ends_with($real_path, '/') && !str_ends_with($real_path, '\\'))) {
        $real_path .= DIRECTORY_SEPARATOR;
    }
    
    if (str_starts_with($real_path, $cbz_dir)) {
        return true;
    } else {
        return false;
    }
}

function check_path_query(): string {
    // ToDo: Also canonicalize
    
    if (!isset($_GET['path'])) {
        http_response_code(400);
        die("`path` parameter is missing");
    }
    
    $path = trim($_GET['path']);
    
    return $path;
}

function get_parent_if_exists(string $virtual_path): string|false {
    $real_path = get_real_path($virtual_path);
    if ($real_path === false) {
        return false;
    }
    $virtual_path = get_virtual_path($real_path);
    if ($virtual_path === false) {
        return false;
    }
    
    $virtual_path = trim($virtual_path);
    $virtual_path = str_replace('\\', '/', $virtual_path);
    $virtual_path = trim($virtual_path, '/');
    
    $split_path = explode('/', $virtual_path);
    
    if (empty($split_path[0])) {
        // No parent dir
        return false;
    }
    
    if (count($split_path) === 1) {
        return '';
    }
    $parent_path = implode('/', array_slice($split_path, 0, -1));
    return $parent_path;
}

function get_extension(string $path): string {
    return strtolower(pathinfo($path, PATHINFO_EXTENSION));
}

function get_comic_data(string $real_path): array|false {
    $extension = get_extension($real_path);

    $ret = [
        'ComicInfo.xml' => false,
        'pages' => [],
        'pageCnt' => 0,
    ];

    switch ($extension) {
        case 'cbz': {
            $cbz = new CbzFile();
            $res = $cbz->open($real_path);
            if ($res === false) {
                return false;
            }
            
            $pages = $cbz->getPages();
            foreach ($pages as $idx => $page) {
                $ret['pages'][] = [
                    'pageNo' => $idx + 1,
                    'imageFile' => $page,
                ];
            }
            $ret['pageCnt'] = count($pages);
            
            $ret['ComicInfo.xml'] = $cbz->getComicInfoXml();

            break;
        }
        default: {
            return false;
        }
    }
    
    return $ret;
}

function init_sentry_tracing(string $endpoint): \Sentry\Tracing\Transaction {
    // Setup context for the full transaction
    $transactionContext = \Sentry\Tracing\TransactionContext::make()
        ->setName($endpoint)
        ->setOp('http.server');
    // Start the transaction
    $transaction = \Sentry\startTransaction($transactionContext);
    // Set the current transaction as the current span so we can retrieve it later
    \Sentry\SentrySdk::getCurrentHub()->setSpan($transaction);
    
    return $transaction;
}

function search_supported_item_in_sub_dirs(string $real_path): string|false {
    if (!is_dir($real_path)) return false;
    $dp = opendir($real_path);
    if ($dp === false) return false;
    
    try {
        while (false !== ($entry = readdir($dp))) {
            if ($entry == '.' || $entry == '..') continue;
            
            $full_path = $real_path . DIRECTORY_SEPARATOR . $entry;
            if (is_dir($full_path)) {
                $sub_dir_res = search_supported_item_in_sub_dirs($full_path);
                if ($sub_dir_res !== false) {
                    return $sub_dir_res;
                }
            } else if (
                is_file($full_path) &&
                in_array(get_extension($full_path), SUPPORTED_ITEM_EXTENSIONS)
            ) {
                return $full_path;
            }
        }
    }
    finally {
        closedir($dp);
    }
    
    return false;
}

function process_last_modified(string $real_path) {
    $last_modified_time = filemtime($real_path);
    if ($last_modified_time === false) {
        return;
    }
    $last_modified_formatted = gmdate('D, d M Y H:i:s', $last_modified_time) . ' GMT';
    header("Last-Modified: $last_modified_formatted");
    
    $if_modified_since = $_SERVER['HTTP_IF_MODIFIED_SINCE'] ?? null;
    
    if ($if_modified_since !== null) {
        $if_modified_since_time = strtotime($if_modified_since);
        if ($if_modified_since_time === false) {
            return;
        }
        
        if ($if_modified_since_time >= $last_modified_time) {
            http_response_code(304);
            exit;
        }
    }
}