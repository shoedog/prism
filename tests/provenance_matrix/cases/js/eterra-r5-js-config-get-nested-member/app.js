const config = require("config");
function run() {
  const settings = config.get("server");
  const port = settings.port;
  console.log(port);
}
