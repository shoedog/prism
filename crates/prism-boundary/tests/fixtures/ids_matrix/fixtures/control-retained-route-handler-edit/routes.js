function send() { return "ochre"; }
route([{ method: "GET", path: "/kiln", handler: send }]);
