function list() { return []; }

module.exports = function register(server, options) {
    server.route([{ method: 'GET', path: '/users', handler: list }]);
};
