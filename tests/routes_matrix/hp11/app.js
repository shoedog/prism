const Hapi = require('@hapi/hapi');

const controller = {
    list() { return []; },
};

function status() { return { ready: true }; }
function wrap(handler) { return (request, h) => handler(request, h); }
function check() { return 'checked'; }

const plugin = {
    name: 'handlers',
    register(server, options) {
        server.decorate('handler', 'directory', (_route, settings) => () => ({
            directory: settings.path,
        }));
        server.route([
            { method: 'GET', path: '/inline', handler: () => 'inline' },
            { method: 'GET', path: '/member', handler: controller.list },
            { method: 'GET', path: '/wrapped', handler: wrap(status) },
            { method: 'GET', path: '/directory', handler: { directory: { path: '.' } } },
            { method: 'GET', path: '/pre', handler: status, options: { pre: [{ method: check }] } },
        ]);
    },
};

const app = Hapi.server();

async function configure() {
    await app.register(plugin);
    return app;
}

module.exports = { app, configure };
