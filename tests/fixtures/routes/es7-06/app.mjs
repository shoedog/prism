import Hapi from '@hapi/hapi';
import { plugin } from './plugin.mjs';

export async function createApp() {
  const server = Hapi.server();
  await server.register(plugin, { routes: { prefix: '/staff' } });
  await server.initialize();
  return server;
}
