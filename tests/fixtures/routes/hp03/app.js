const Hapi = require('@hapi/hapi');

function list() { return []; }
function create(_request, h) { return h.response({ created: true }).code(201); }

const plugin = {
    name: 'users',
    register: async (server, options) => {
        server.route([
            { method: 'GET', path: '/users', handler: list },
            { method: 'POST', path: '/users', config: { handler: create } },
        ]);
    },
};

const app = Hapi.server();

async function configure() {
    await app.register(plugin, { routes: { prefix: '/api' } });
    return app;
}

module.exports = { app, configure };
