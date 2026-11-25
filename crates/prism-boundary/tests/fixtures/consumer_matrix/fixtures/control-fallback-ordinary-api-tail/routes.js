const routes = [
  { method: "GET", path: "/api/a", handler: () => "amber" },
  { method: "GET", path: "/api/{*0+}", handler: () => "ochre" },
];
