const Hapi = require('@hapi/hapi');

function status() { return { ready: true }; }

const plugin = {
    name: 'health',
    register(server, options) {
        const other = { route(rows) { return rows.length; } };
        {
            const server = other;
            server.route([{ method: 'GET', path: '/block', handler: status }]);
        }
        const install = (server) => server.route([
            { method: 'GET', path: '/callback', handler: status },
        ]);
        install(other);
    },
};

const app = Hapi.server();

async function configure() {
    await app.register(plugin);
    return app;
}

module.exports = { app, configure };
