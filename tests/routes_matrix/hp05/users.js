function list() { return []; }

async function register(server, options) {
    server.route([{ method: 'GET', path: '/users', handler: list }]);
}

module.exports = { name: 'u', register };
