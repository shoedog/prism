const Hapi = require('@hapi/hapi');

function inventory() {
  return [];
}

const stock = {
  name: 'stock',
  register(api) {
    api.dependency('warehouse', () => {
      api.route({ method: 'GET', path: '/inventory', handler: inventory });
    });
  },
};

const warehouse = {
  name: 'warehouse',
  register(storage) {
    storage.expose('aisleCount', 3);
  },
};

module.exports = async function createApp() {
  const app = Hapi.server();
  await app.register(stock, { routes: { prefix: '/stock' } });
  await app.register(warehouse);
  await app.initialize();
  return app;
};
