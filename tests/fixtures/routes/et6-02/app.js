const express = require('express');
const app = express();
const router = express.Router();
app.use('/v1', router);
function list(req, res) { res.json([]); }
router.get('/items', list);
app.use('/v2', router);
module.exports = app;
