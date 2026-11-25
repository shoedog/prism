const express = require('express');
const app = express();
const router = express.Router();
function list(req, res) { res.end('list'); }
router.get('/items', list);
app.use(router);
module.exports = app;
