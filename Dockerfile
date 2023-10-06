FROM composer AS installdep

COPY composer.json /app/

COPY --from=composer /usr/bin/composer /usr/bin/composer

RUN composer install

FROM php:8.2-apache

RUN mv "$PHP_INI_DIR/php.ini-production" "$PHP_INI_DIR/php.ini"

COPY --from=installdep /app /var/www/html

COPY _config.docker.php /var/www/html/_config.php

COPY _shared.php /var/www/html/_shared.php
COPY img.php /var/www/html/img.php
COPY list.php /var/www/html/list.php
COPY read.php /var/www/html/read.php