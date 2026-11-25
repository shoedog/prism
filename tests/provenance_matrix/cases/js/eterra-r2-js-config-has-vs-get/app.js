const config = require("config");
function run() {
  const present = config.has("mode");
  const mode = config.get("mode");
  console.log(present, mode);
}
