const Hapi = require('@hapi/hapi');
const app = Hapi.server();
function listing() { return []; }
const plugin = {
  name: 'preview',
  register(api) {
    api.route({ method: 'GET', path: '/preview', handler: listing });
  }
};

const descriptors = [];
const catalog = {
  register(item) {
    item.register({ route(row) { descriptors.push(row); } });
  }
};
catalog.register(plugin);
module.exports = app;
