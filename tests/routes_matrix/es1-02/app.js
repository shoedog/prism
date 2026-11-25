const express = require('express');
const app = express();
const items = express.Router();
const archive = express.Router({ mergeParams: true });
function list(req, res) { res.send('items'); }
items.get('/items', list);
app.use('/api', items);
items.use('/archive', archive);
archive.use('/current', items);
module.exports = app;
