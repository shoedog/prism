const Hapi = require("@hapi/hapi");
const server = Hapi.server();

const plugin = {
  name: "pottery",
  register(pluginServer) {
    pluginServer.route([
      { method: "POST", path: "/mugs", handler: () => "glaze" },
    ]);
  },
};

server.register(plugin, { routes: { prefix: "/atelier" } });
