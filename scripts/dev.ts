import { consola } from 'consola';
import { createServer } from 'node:net';
import { fileURLToPath } from 'node:url';
import * as v from 'valibot';

const portSchema = v.optional(v.pipe(v.string(), v.regex(/^\d+$/), v.transform(Number)));
const nuxtPort = v.parse(portSchema, process.env.NUXT_PORT) ?? 3000;

const availablePort = await findAvailablePort(nuxtPort);

if (availablePort !== nuxtPort) {
  consola.info(`Port ${nuxtPort} is busy, using port ${availablePort}`);
}

const tauriConfig = JSON.stringify({
  build: {
    devUrl: `http://localhost:${availablePort}`,
    beforeDevCommand: `bun run nuxt:dev --port ${availablePort}`,
  },
});

const tauri = Bun.spawn(['bun', 'run', 'tauri', 'dev', '--config', tauriConfig], {
  cwd: fileURLToPath(new URL('..', import.meta.url)),
  stdio: ['inherit', 'inherit', 'inherit'],
});

process.on('SIGINT', () => {
  tauri.kill();
});

process.on('SIGTERM', () => {
  tauri.kill();
});

process.exitCode = await tauri.exited;

function isPortAvailable(port: number) {
  return new Promise((resolve) => {
    const server = createServer();

    server.once('error', () => {
      resolve(false);
    });

    server.once('listening', () => {
      server.close(() => {
        resolve(true);
      });
    });

    server.listen(port, '0.0.0.0');
  });
}

async function findAvailablePort(startPort: number) {
  const ports = Array.from({ length: 100 }, (_, index) => startPort + index);
  const availability = await Promise.all(ports.map((port) => isPortAvailable(port)));
  const port = ports[availability.findIndex(Boolean)];

  if (port === undefined) {
    throw new Error(`No available port found in range ${startPort}-${startPort + 99}`);
  }

  return port;
}
