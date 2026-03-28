<?php
require_once __DIR__ . '/../internals/_init.php';

use comic_format\CbzFile;

$transaction = init_sentry_tracing('/api/img');

$path = check_path_query();

if (!isset($_GET['f'])) {
    http_response_code(400);
    die('No required parameter found: f');
}
$f = trim($_GET['f']);
if (empty($f)) {
    http_response_code(400);
    die('No required parameter found: f. Empty is not allowed.');
}

$thumb = isset($_GET['thumb']);

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

process_last_modified($real_path);

$extension = get_extension($real_path);
$inner_extension = get_extension($f);

$image_content_type = false;
$image_content = null;

switch ($extension) {
    case 'cbz': {
        $cbz = new CbzFile();
        $res = $cbz->open($real_path);
        if ($res === false) {
            http_response_code(500);
            die('Unable to open cbz file');
        }
        
        if (!$cbz->isFile($f)) {
            http_response_code(404);
            die('Internal file is invalid: not found');
        }

        $image_content = $cbz->readImage($f);
        if ($image_content === false) {
            http_response_code(500);
            die('Internal file is invalid: unable to read');
        }
        
        $image_content_type = get_mime_type_from_extension($inner_extension);

        break;
    }
    default: {
        http_response_code(404);
        die('This file is not supported.');
    }
}

if (!$thumb && $image_content_type !== false) {
    header('Content-type: ' . $image_content_type);
    header('Cache-Control: public, max-age=31536000');
    print($image_content);
    $transaction->finish();
    exit;
}

$image = imagecreatefromstring($image_content);
unset($image_content);

if ($thumb) {
    $new_size = 160;
    $new_size_f = 160.0;
    
    $orig_width = imagesx($image);
    $orig_height = imagesy($image);

    $orig_width_f = floatval($orig_width);
    $orig_height_f = floatval($orig_height);
    
    $do_resize = false;
    
    $new_width = $new_size;
    $new_height = $new_size;

    if ($orig_width > $orig_height) {
        if ($orig_width > $new_size) {
            $ratio = $new_size_f / $orig_width_f;
            $new_height = intval($orig_height_f * $ratio);
            $do_resize = true;
        }
    } else {
        if ($orig_height > $new_size) {
            $ratio = $new_size_f / $orig_height_f;
            $new_width = intval($orig_width_f * $ratio);
            $do_resize = true;
        }
    }
    
    if ($do_resize) {
        $resized = imagecreatetruecolor($new_width, $new_height);
        imagecopyresampled($resized, $image, 0, 0, 0, 0, $new_width, $new_height, $orig_width, $orig_height);
        unset($image);
        $image = $resized;
    }
}

$quality = $thumb ? 20 : 80;

header('Content-type: ' . $image_content_type);
header('Cache-Control: public, max-age=31536000');
$res = imagejpeg($image, null, $quality);
if ($res === false) {
    http_response_code(500);
    die('Unable to encode image');
}

$transaction->finish();