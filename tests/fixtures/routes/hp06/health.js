function ready() { return { ready: true }; }

export default {
    name: 'health',
    register(server, options) {
        server.route([{ method: 'GET', path: '/health', handler: ready }]);
    },
};
