const Hapi = require('@hapi/hapi');
const app = Hapi.server();
async function configure() {
    await app.register(require('./inventory').plugin, { routes: { prefix: '/shop' } });
    return app;
}
module.exports = { app, configure };
