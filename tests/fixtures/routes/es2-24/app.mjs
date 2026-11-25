import Hapi from '@hapi/hapi';
import { plugin as queues } from './queues.mjs';

const app = Hapi.server();
await app.register(queues, { routes: { prefix: '/ops' } });

export default app;
