const { Client } = require("pg");
async function run() {
  const client = new Client();
  const connected = await client.connect();
  const rows = await client.query("select 1");
  console.log(connected, rows);
}
