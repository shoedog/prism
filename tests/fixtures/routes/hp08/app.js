const Hapi = require('@hapi/hapi');

function list() { return []; }

const plugin = {
    name: 'users',
    multiple: true,
    register(server, options) {
        server.route([{ method: 'GET', path: '/users', handler: list }]);
    },
};

const app = Hapi.server();

async function configure() {
    await app.register(plugin, { routes: { prefix: '/v1' } });
    await app.register(plugin, { routes: { prefix: '/v2' } });
    return app;
}

module.exports = { app, configure };
