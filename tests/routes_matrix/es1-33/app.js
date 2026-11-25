const Hapi = require('@hapi/hapi');
const app = Hapi.server();
const collector = { route() {} };
function queue() { return []; }
const plugin = {
  name: 'preview',
  register(api) {
    ({ target: api } = { target: collector });
    api.route({ method: 'GET', path: '/queue', handler: queue });
  }
};
async function build() {
  await app.register(plugin);
  return app;
}
module.exports = build;
