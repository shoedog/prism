const { publicBase } = require("./public-base");
const routes = [];
function route(entries) {
  routes.push(...entries.map((entry) => ({ ...entry, path: publicBase + entry.path })));
}
route([
  { method: "POST", path: "/mugs", handler: () => "1" },
]);
