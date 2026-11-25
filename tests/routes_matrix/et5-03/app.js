const Hapi = require('@hapi/hapi');
const app = Hapi.server();
async function configure() {
    await app.register(require('./outer'), { routes: { prefix: '/api' } });
    return app;
}
module.exports = { app, configure };
