const { Pool } = require("pg");

async function load() {
  const db = new Pool();
  const sql = "select 1";
  const result = await db.query(sql);
  return result;
}
