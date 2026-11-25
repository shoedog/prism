const pg = require('pg');
async function listen(deliver) {
  const client = new pg.Client();
  await client.connect();
  client.on('notification', message => {
    const edition = message.payload;
    const publisher = message.processId;
    deliver(edition, publisher);
  });
  await client.query('LISTEN editions');
  return client;
}
