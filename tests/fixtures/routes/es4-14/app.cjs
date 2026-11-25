const Hapi = require('@hapi/hapi');

function ready() { return 'ready'; }
function preview() { return 'preview'; }

const plugin = {
  name: 'reports',
  register(pluginServer) {
    function enablePreview() {
      pluginServer.route({ method: 'GET', path: '/preview', handler: preview });
    }
    pluginServer.expose('enablePreview', enablePreview);
    pluginServer.route({ method: 'GET', path: '/ready', handler: ready });
  },
};

async function createApp() {
  const app = Hapi.server();
  await app.register(plugin, { routes: { prefix: '/reports' } });
  return app;
}

module.exports = { createApp };
