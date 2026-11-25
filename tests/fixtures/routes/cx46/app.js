const Hapi = require('@hapi/hapi');

const app = Hapi.server();

function status() { return 'ready'; }

function install(pluginServer) {
    pluginServer.route({ method: 'GET', path: '/readiness', handler: status });
}

function decorate(register) { return register; }

const plugin = { name: 'readiness', register: decorate(install) };

async function configure() {
    await app.register(plugin);
    return app;
}

module.exports = { app, configure };
