const express = require('express');
const app = express();

function readLedger(req, res) { res.send('ledger'); }
function checkLedger(req, res) { res.status(204).end(); }
function readManifest(req, res) { res.send('manifest'); }
function checkManifest(req, res) { res.status(204).end(); }

app.get('/ledger', readLedger);
app.head('/ledger', checkLedger);
app.route('/manifest').get(readManifest).head(checkManifest);

module.exports = app;
