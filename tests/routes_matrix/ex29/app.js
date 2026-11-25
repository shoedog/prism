const express = require('express');
const app = express();

function live(req, res) { res.sendStatus(204); }
function ghost(req, res) { res.sendStatus(204); }

let r = express.Router();
r.get('/live', live);
app.use('/api', r);

r = express.Router({ mergeParams: true });
r.get('/ghost', ghost);

module.exports = app;
