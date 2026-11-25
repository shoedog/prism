const Hapi = require('@hapi/hapi');

function status() { return { ready: true }; }
function reports() { return []; }

const plugin = {
    name: 'reports',
    register(server, options) {
        server.route([
            { method: '*', path: '/status', handler: status },
            { method: ['GET', 'POST'], path: '/reports', handler: reports },
        ]);
    },
};

const app = Hapi.server();

async function configure() {
    await app.register(plugin);
    return app;
}

module.exports = { app, configure };
