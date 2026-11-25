const Hapi = require("@hapi/hapi");
const server = Hapi.server();

server.route([
  { method: "POST", path: "/mugs/{trail*2}", handler: () => "1" },
]);
