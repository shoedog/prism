import * as Hapi from '@hapi/hapi';
import * as reports from './reports.mjs';
const app = Hapi.server();
export async function configure() {
    await app.register(reports.plugin, { routes: { prefix: '/v1' } });
    return app;
}
export { app };
