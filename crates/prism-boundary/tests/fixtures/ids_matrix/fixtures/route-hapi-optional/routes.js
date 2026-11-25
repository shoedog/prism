const Hapi = require("@hapi/hapi");
const server = Hapi.server();

server.route([
  { method: "POST", path: "/mugs/{size?}", handler: () => "1" },
]);
