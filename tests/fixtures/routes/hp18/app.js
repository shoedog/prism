const Hapi = require('@hapi/hapi');

function status() { return { ready: true }; }
function wrap(target) { return { route(rows) { return target.route(rows); } }; }

const destructured = {
    name: 'destructured',
    register({ route }, options) {
        route([{ method: 'GET', path: '/destructured', handler: status }]);
    },
};

const reassigned = {
    name: 'reassigned',
    register(server) {
        server = wrap(server);
        server.route([{ method: 'GET', path: '/written', handler: status }]);
    },
};

const app = Hapi.server();

async function configure() {
    await app.register([destructured, reassigned]);
    return app;
}

module.exports = { app, configure };
