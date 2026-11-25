function ping() { return { ok: true }; }

module.exports = {
    pkg: { name: 'monitor', version: '1.0.0' },
    register(pluginServer) {
        pluginServer.route({ method: 'GET', path: '/monitor', handler: ping });
    },
};
