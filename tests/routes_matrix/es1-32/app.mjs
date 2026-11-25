import Hapi from '@hapi/hapi';
import { rates } from './index.mjs';

const app = Hapi.server();
export default async function build() {
  await app.register(rates, { routes: { prefix: '/v2' } });
  return app;
}
