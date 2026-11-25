const Hapi = require('@hapi/hapi');

function list() { return []; }

const plugin = {
    name: 'users',
    register(server, options) {
        server.route([{ method: 'GET', path: '/users', handler: list }]);
    },
};

const registry = {
    register(item) { this.item = item; },
};
registry.register(plugin);

const app = Hapi.server();

module.exports = app;
