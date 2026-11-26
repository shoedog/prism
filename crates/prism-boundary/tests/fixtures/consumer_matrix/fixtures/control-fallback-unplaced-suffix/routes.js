const routes = [
  { method: "GET", path: "/api/a", handler: () => "amber" },
  { method: "GET", path: "/{*0+}", handler: () => "ochre" },
];
const kiln = [{ method: "GET", path: "/b", handler: () => "glaze" }];
