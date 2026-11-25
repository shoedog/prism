function list() { return []; }

module.exports = {
    register(server) {
        server.route([{ method: 'GET', path: '/users', handler: list }]);
    },
};
