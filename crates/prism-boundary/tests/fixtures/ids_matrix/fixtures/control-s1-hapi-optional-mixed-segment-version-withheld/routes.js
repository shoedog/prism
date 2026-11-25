// Hapi 21.3.12
const Hapi = require("@hapi/hapi");
const server = Hapi.server();

server.route([
  { method: "POST", path: "/fletch/wax-{hue?}/proof", handler: () => "amber" },
]);
