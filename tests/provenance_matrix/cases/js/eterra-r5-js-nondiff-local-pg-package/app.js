const pg = require("./pg");
async function run() {
  const result = await pg.query();
  console.log(result);
}
