<?php

namespace comic_format;

interface ComicFormat
{
    public function open(string $filePath): bool;
    public function getComicInfoXml(): string|false;
    public function getPages(): array;
    public function isFile(string $filePath): bool;
    public function readImage(string $imagePath): string|false;
}