const Hapi = require('@hapi/hapi');

const base = process.env.API_PREFIX ?? '/api';
const PATH = process.env.REPORTS_PATH ?? '/reports';

function status() { return { ready: true }; }
function reports() { return []; }

const plugin = {
    name: 'reports',
    register(server, options) {
        server.route([
            { method: 'GET', path: '/x', handler: status },
            { method: 'GET', path: PATH, handler: reports },
        ]);
    },
};

const app = Hapi.server();

async function configure() {
    await app.register(plugin, { routes: { prefix: base } });
    return app;
}

module.exports = { app, configure };
