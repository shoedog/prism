const pg = require("pg");
pg.Pool = class { async query() { return 3; } };
async function run() {
  const pool = new pg.Pool();
  const result = await pool.query("select 1");
  console.log(result);
}
