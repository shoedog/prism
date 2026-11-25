const Hapi = require('@hapi/hapi');

function status() { return { ready: true }; }

const plugin = {
    name: 'health',
    register(server, options) {
        server.route({ method: 'GET', path: '/x', options: { handler: status } });
    },
};

const app = Hapi.server();

async function configure() {
    await app.register(plugin);
    return app;
}

module.exports = { app, configure };
