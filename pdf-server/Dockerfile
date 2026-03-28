FROM oven/bun:1 AS base
WORKDIR /usr/src/app

COPY package.json bun.lock tsconfig.json ./
RUN bun install --frozen-lockfile

COPY *.ts ./

ENV NODE_ENV=production

CMD ["bun", "--preload", "./instrument.ts", "index.ts"]