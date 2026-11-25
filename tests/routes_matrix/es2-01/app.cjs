const express = require('express');
const app = express();

function receipt(req, res) { res.sendStatus(204); }

app.acl('/receipts/acl', receipt);
app.checkout('/receipts/checkout', receipt);
app.connect('/receipts/connect', receipt);
app.copy('/receipts/copy', receipt);
app.link('/receipts/link', receipt);
app.lock('/receipts/lock', receipt);
app.merge('/receipts/merge', receipt);
app.mkactivity('/receipts/mkactivity', receipt);
app.mkcalendar('/receipts/mkcalendar', receipt);
app.mkcol('/receipts/mkcol', receipt);
app.move('/receipts/move', receipt);
app.notify('/receipts/notify', receipt);
app.propfind('/receipts/propfind', receipt);
app.proppatch('/receipts/proppatch', receipt);
app.purge('/receipts/purge', receipt);
app.query('/receipts/query', receipt);
app.rebind('/receipts/rebind', receipt);
app.report('/receipts/report', receipt);
app.source('/receipts/source', receipt);
app.subscribe('/receipts/subscribe', receipt);
app.trace('/receipts/trace', receipt);
app.unbind('/receipts/unbind', receipt);
app.unlink('/receipts/unlink', receipt);
app.unlock('/receipts/unlock', receipt);
app.unsubscribe('/receipts/unsubscribe', receipt);

module.exports = app;
