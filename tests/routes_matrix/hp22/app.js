const Hapi = require('@hapi/hapi');

function list() { return []; }
function status() { return { ready: true }; }

const users = {
    name: 'users',
    register(server, options) {
        server.route([{ method: 'GET', path: '/users', handler: list }]);
    },
};

const health = {
    name: 'health',
    register(server, options) {
        server.route([{ method: 'GET', path: '/health', handler: status }]);
    },
};

const plugins = [users, health];
const app = Hapi.server();

async function configure() {
    for (const p of plugins) {
        await app.register(p);
    }
    return app;
}

module.exports = { app, configure };
