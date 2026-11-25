const Hapi = require('@hapi/hapi');

const app = Hapi.server();

function ping() { return { ok: true }; }

const plugin = {
    name: 'clients',
    register(pluginServer) {
        pluginServer.route({ method: 'GET', path: '/clients', handler: ping });
    },
};

function vendorStatus() { return { ok: true }; }

const vendors = {
    name: 'vendors',
    register(vendorServer) {
        vendorServer.route({ method: 'GET', path: '/vendors', handler: vendorStatus });
    },
};

async function configure() {
    await app.register([plugin, { plugin: vendors, routes: { prefix: '/staff' } }]);
    return app;
}

module.exports = { app, configure };
