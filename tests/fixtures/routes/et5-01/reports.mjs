function list() { return []; }
export const plugin = {
    name: 'reports',
    register(server) {
        server.route({ method: 'GET', path: '/reports', handler: list });
    },
};
