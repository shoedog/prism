const Hapi = require("@hapi/hapi");
const server = Hapi.server({ router: { stripTrailingSlash: true } });

server.route([
  { method: "POST", path: "/mugs", handler: () => "glaze" },
]);
