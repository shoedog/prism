const Hapi = require("@hapi/hapi");
const server = Hapi.server();

server.route([
  { method: ["post", "options"], path: "/mugs", handler: () => "1" },
]);
