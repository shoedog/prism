// Hapi 21.3.12
const Hapi = require("@hapi/hapi");
const server = Hapi.server();

server.route([
  { method: "POST", path: "/fletch/internal", options: { isInternal: true }, handler: () => "amber" },
  { method: "POST", path: "/fletch/public", handler: () => "ochre" },
]);
