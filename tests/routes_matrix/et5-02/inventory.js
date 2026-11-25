function list() { return []; }
module.exports.plugin = {
    name: 'inventory',
    register(server) {
        server.route({ method: 'GET', path: '/items', handler: list });
    },
};
