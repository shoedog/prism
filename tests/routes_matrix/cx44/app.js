const Hapi = require('@hapi/hapi');

const app = Hapi.server();



async function configure() {
    await app.register(require('./monitor'));
    return app;
}

module.exports = { app, configure };
