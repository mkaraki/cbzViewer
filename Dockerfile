FROM composer AS require-server

WORKDIR /app

COPY composer.json composer.lock /app/

RUN composer install --ignore-platform-reqs

FROM oven/bun:latest AS frontend

WORKDIR /app

COPY frontend/package.json frontend/bun.lock /app/

RUN bun install

COPY --exclude=dist --exclude=node_modules frontend /app

RUN --mount=type=secret,id=SENTRY_ORG,env=SENTRY_ORG \
    --mount=type=secret,id=SENTRY_PROJECT,env=SENTRY_PROJECT \
    --mount=type=secret,id=SENTRY_AUTH_TOKEN,env=SENTRY_AUTH_TOKEN \
    --mount=type=secret,id=SENTRY_URL,env=SENTRY_URL \
    bun run build

FROM dunglas/frankenphp:php8.5

RUN install-php-extensions \
	excimer \
	gd \
	zip \
	opcache \
    apcu

COPY --from=require-server /app/vendor /app/public/vendor
COPY --from=frontend /app/dist /app/public
COPY Caddyfile /etc/frankenphp/Caddyfile

ARG USER=appuser
RUN \
	useradd ${USER}; \
	setcap -r /usr/local/bin/frankenphp; \
	chown -R ${USER}:${USER} /config/caddy /data/caddy
USER ${USER}

COPY config.docker.json /app/public/config.json
COPY *.php /app/public/
COPY api internals /app/public/

EXPOSE 8080
