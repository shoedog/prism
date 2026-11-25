function send() { return "amber"; }
route([{ method: "GET", path: "/kiln", handler: send }]);
