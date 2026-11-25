const Hapi = require('@hapi/hapi');

function list() { return []; }
function fallback() { return ['fallback']; }

const plugin = {
    name: 'users',
    register(server, options) {
        server.route([
            { method: 'GET', path: '/duplicate', handler: list, config: { handler: fallback } },
            { method: 'GET', path: '/missing' },
        ]);
    },
};

const app = Hapi.server();

async function configure() {
    await app.register(plugin);
    return app;
}

module.exports = { app, configure };
