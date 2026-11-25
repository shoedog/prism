const pg = require("pg");
async function run() {
  const client = new pg.Client();
  const rows = await client.query("select 1");
  const quoted = pg.escapeIdentifier("table");
  console.log(rows, quoted);
}
