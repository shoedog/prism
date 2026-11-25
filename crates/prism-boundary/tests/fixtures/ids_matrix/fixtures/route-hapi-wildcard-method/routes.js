const Hapi = require("@hapi/hapi");
const server = Hapi.server();

server.route([
  { method: "*", path: "/mugs", handler: () => "1" },
]);
