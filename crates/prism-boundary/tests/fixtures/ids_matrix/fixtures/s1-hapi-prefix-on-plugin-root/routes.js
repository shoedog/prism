// Hapi 21.3.12
const Hapi = require("@hapi/hapi");
const server = Hapi.server();

const spindle = {
  name: "moonspindle",
  register(scoped) {
    scoped.route([
      { method: "POST", path: "/", handler: () => "amber" },
    ]);
  },
};

async function makeServer() {
  await server.register(spindle, { routes: { prefix: "/fletch" } });
  return server;
}

module.exports = makeServer;
