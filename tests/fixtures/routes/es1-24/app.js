const Hapi = require('@hapi/hapi');
const app = Hapi.server();

function members() { return []; }
const install = (api, options) => {
  api.route({ method: 'GET', path: '/members', handler: members });
};
const plugin = { name: 'directory', register: install };
const registration = { plugin, routes: { prefix: '/team' } };

async function build() {
  await app.register(registration);
  return app;
}
module.exports = build;
