const Hapi = require("@hapi/hapi");
const server = Hapi.server({ router: { isCaseSensitive: false } });

server.route([
  { method: "POST", path: "/Mugs", handler: () => "1" },
]);
