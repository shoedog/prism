route([
  { method: "HEAD", path: "/mugs", handler: () => "1" },
  { method: "OPTIONS", path: "/mugs", handler: () => "2" },
]);
