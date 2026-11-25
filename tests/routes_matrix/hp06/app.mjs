import Hapi from '@hapi/hapi';
import { plugin as users } from './users.js';
import health from './health.js';

const app = Hapi.server();

export async function configure() {
    await app.register(users);
    await app.register(health);
    return app;
}

export { app };
