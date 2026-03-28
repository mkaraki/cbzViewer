<?php
namespace comic_format;

require_once __DIR__ . '/../_init.php';

use comic_format\ComicFormat;
use ZipArchive;

class CbzFile implements ComicFormat
{
    private $zip_archive;
    
    public function __construct() {
        $this->zip_archive = new ZipArchive();
    }

    public function __destruct() {
        $this->zip_archive->close();
    }
    
    public function open(string $filePath): bool {
        $res = $this->zip_archive->open($filePath, ZipArchive::CREATE);
        return $res === true;
    }

    public function getComicInfoXml(): string|false
    {
        return $this->zip_archive->getFromName("ComicInfo.xml");
    }

    public function getPages(): array
    {
        $pageList = [];
        
        for ($i = 0; $i < $this->zip_archive->numFiles; $i++) {
            $name = $this->zip_archive->getNameIndex($i);
            if ($name === false) continue;
            
            $ext = get_extension($name);
            if (in_array($ext, SUPPORTED_IMAGE_EXTENSIONS))
                $pageList[] = $name;
        }
        
        natsort($pageList);
        return $pageList;
    }
    
    public function isFile(string $filePath): bool
    {
        $stat = $this->zip_archive->statName($filePath);
        return $stat !== false;
    }

    public function readImage(string $imagePath): string|false
    {
        if (!$this->isFile($imagePath)) {
            return false;
        }
        
        $f = $this->zip_archive->getFromName($imagePath);
        return $f;
    }
}