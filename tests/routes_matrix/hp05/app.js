const Hapi = require('@hapi/hapi');

const app = Hapi.server();

async function configure() {
    await app.register({ plugin: require('./users'), routes: { prefix: '/v1' } });
    return app;
}

module.exports = { app, configure };
