function list() { return []; }

export const plugin = {
    name: 'users',
    register(server, options) {
        server.route([{ method: 'GET', path: '/users', handler: list }]);
    },
};
