const Hapi = require('@hapi/hapi');

function status() { return { ready: true }; }

const outer = {
    name: 'outer',
    async register(server, options) {
        await server.register(inner, { routes: { prefix: '/b' } });
    },
};

const inner = {
    name: 'inner',
    register(childServer, options) {
        childServer.route([{ method: 'GET', path: '/x', handler: status }]);
    },
};

const app = Hapi.server();

async function configure() {
    await app.register(outer, { routes: { prefix: '/a' } });
    return app;
}

module.exports = { app, configure };
