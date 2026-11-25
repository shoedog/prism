const express = require('express');

const app = express();
const catalog = express.Router({ mergeParams: true });
const inventory = express.Router();

function stock(req, res) { res.json({ inStock: true }); }

inventory.get('/stock', stock);
catalog.use(['/live', '/preview'], inventory);
app.use('/api', catalog);

module.exports = app;
