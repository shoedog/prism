const Hapi = require('@hapi/hapi');

function roster() {
  return [];
}

const plugin = {
  name: 'staff-roster',
  register(api) {
    api.ext('onPreStart', () => {
      api.route({ method: 'GET', path: '/roster', handler: roster });
    });
  },
};

module.exports = async function createApp() {
  const app = Hapi.server();
  await app.register(plugin, { routes: { prefix: '/staff' } });
  await app.initialize();
  return app;
};
