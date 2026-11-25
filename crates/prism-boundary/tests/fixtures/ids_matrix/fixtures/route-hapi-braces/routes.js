const Hapi = require("@hapi/hapi");
const server = Hapi.server();

server.route([
  { method: "GET", path: "/mugs/{size}", handler: () => "1" },
]);
