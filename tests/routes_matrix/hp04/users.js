const name = 'users';

function list() { return []; }

async function register(server, options) {
    server.route([{ method: 'GET', path: '/users', handler: list }]);
}

exports.plugin = { name, register };
