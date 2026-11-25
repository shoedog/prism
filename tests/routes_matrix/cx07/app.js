const Hapi = require('@hapi/hapi');

const app = Hapi.server();

function ping() { return { ok: true }; }

const plugin = {
    name: 'checks',
    register(pluginServer) {
        pluginServer.route({ method: 'GET', path: '/checks', handler: ping });
    },
};

function makePlugins() { return [plugin]; }

async function configure() {
    await app.register(makePlugins());
    return app;
}

module.exports = { app, configure };
