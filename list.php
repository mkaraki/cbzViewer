<?php
require_once __DIR__ . '/_shared.php';
if (!is_dir(DATA_QUERY_PATH)) {
    die('Not a directory');
}
?>
<!DOCTYPE html>
<html lang="en">

<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>ls</title>
    <style>
        ul {
            list-style: none;
            padding-left: 10px;
        }
    </style>
</head>

<body>
    <ul>
        <?php if ($path !== '/') : ?>
            <li>
                <a href="list.php?path=<?= urlencode(str_replace('\\', '/', dirname($path))) ?>">
                    ../
                </a>
            </li>
        <?php endif; ?>
        <?php foreach (new DirectoryIterator(DATA_QUERY_PATH) as $file) : ?>
            <?php
            $relPath = str_replace('\\', '/', substr($file->getPathname(), strlen(DATA_ROOT_ABSOLUTE)));
            $fileName = $file->getFilename();
            if ($fileName === '.' || $fileName === '..') continue;
            ?>
            <li>
                <?php if ($file->isDir()) : ?>
                    📂
                    <a href="list.php?path=<?= urlencode($relPath) ?>">
                        <?= $fileName ?>/
                    </a>
                <?php else : ?>
                    📄
                    <a href="read.php?path=<?= urlencode($relPath) ?>">
                        <?= $fileName ?>
                    </a>
                <?php endif; ?>
            </li>
        <?php endforeach; ?>
    </ul>
</body>

</html>