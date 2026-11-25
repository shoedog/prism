const fs = require("node:fs");

function read() {
  const text = fs.readFileSync("./settings.json", "utf8");
  const config = JSON.parse(text);
  return config;
}
