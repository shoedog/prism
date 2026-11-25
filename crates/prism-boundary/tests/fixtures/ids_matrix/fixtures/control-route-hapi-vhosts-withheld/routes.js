const Hapi = require("@hapi/hapi");
const server = Hapi.server();

server.route([
  { method: "POST", path: "/mugs", vhost: "ash.creel.invalid", handler: () => "1" },
  { method: "POST", path: "/mugs", vhost: "salt.creel.invalid", handler: () => "2" },
]);
